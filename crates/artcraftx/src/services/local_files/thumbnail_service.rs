//! Thumbnails for local files, content-addressed and cached.
//!
//! The routine: stat the file → get its BLAKE3 (from the local_files index
//! when size+mtime match, else rehash) → serve `cache/thumbnails/{hash}.jpg`
//! (static) and `{hash}.webp` (~1s animated preview, videos only),
//! generating them on first sight. Everything on disk is regenerable; the
//! database rows just make the lookups cheap (no re-reading files that
//! haven't changed).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::UNIX_EPOCH;

use file_hashing::hash_file_blake3;
use local_files_database::queries::get_local_file_by_path::get_local_file_by_path;
use local_files_database::queries::set_thumbnail_flags::set_thumbnail_flags;
use local_files_database::queries::upsert_local_file::{upsert_local_file, UpsertLocalFileArgs};
use log::{info, warn};
use once_cell::sync::Lazy;
use serde_derive::Serialize;
use thumbnails::{generate_image_thumbnail, generate_video_thumbnails, ThumbnailError, STATIC_THUMBNAIL_MAX_DIMENSION};
use tokio::sync::{Mutex as TokioMutex, Semaphore};

use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::database::local_files_database::LocalFilesDatabase;

/// Cap on concurrent thumbnail generations (decode + encode are CPU-heavy).
const MAX_CONCURRENT_GENERATIONS: usize = 2;

static GENERATION_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(MAX_CONCURRENT_GENERATIONS));

/// Per-path locks so overlapping bulk requests don't hash or generate the
/// same file twice.
static PER_PATH_LOCKS: Lazy<StdMutex<HashMap<PathBuf, Arc<TokioMutex<()>>>>> =
    Lazy::new(|| StdMutex::new(HashMap::new()));

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp", "tiff", "avif"];
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "m4v", "mov"];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalThumbnailStatus {
  /// A static thumbnail exists (animated preview too, when the path is set).
  Ready,
  /// The file type (or codec) can't be thumbnailed; show a placeholder.
  Unsupported,
  /// The source file no longer exists on disk.
  MissingFile,
  /// Generation errored; a retry may or may not help.
  Failed,
}

#[derive(Clone, Debug, Serialize)]
pub struct LocalThumbnail {
  pub file_path: String,
  pub status: LocalThumbnailStatus,
  pub maybe_file_hash: Option<String>,
  pub maybe_thumbnail_path: Option<String>,
  pub maybe_animated_preview_path: Option<String>,
}

impl LocalThumbnail {
  fn without_thumbnail(file_path: &Path, status: LocalThumbnailStatus, maybe_file_hash: Option<String>) -> Self {
    Self {
      file_path: file_path.to_string_lossy().into_owned(),
      status,
      maybe_file_hash,
      maybe_thumbnail_path: None,
      maybe_animated_preview_path: None,
    }
  }
}

/// Get (generating if needed) the cached thumbnails for one local file.
pub async fn get_or_generate_thumbnail(
  database: &LocalFilesDatabase,
  app_data_root: &AppDataRoot,
  file_path: &Path,
) -> LocalThumbnail {
  // Serialize work per path; overlapping calls wait, then hit the cache.
  let path_lock = per_path_lock(file_path);
  let _path_guard = path_lock.lock().await;

  let stat = match std::fs::metadata(file_path) {
    Ok(metadata) if metadata.is_file() => metadata,
    _ => return LocalThumbnail::without_thumbnail(file_path, LocalThumbnailStatus::MissingFile, None),
  };
  let file_size_bytes = stat.len() as i64;
  let file_mtime_ms = stat.modified().ok()
      .and_then(|mtime| mtime.duration_since(UNIX_EPOCH).ok())
      .map(|duration| duration.as_millis() as i64)
      .unwrap_or(0);

  let file_hash = match cached_or_computed_hash(database, file_path, file_size_bytes, file_mtime_ms).await {
    Ok(hash) => hash,
    Err(message) => {
      warn!("Could not hash {}: {}", file_path.display(), message);
      return LocalThumbnail::without_thumbnail(file_path, LocalThumbnailStatus::Failed, None);
    }
  };

  let thumbnails_dir = match app_data_root.cache_dir().get_or_create_thumbnails_dir() {
    Ok(dir) => dir,
    Err(err) => {
      warn!("Could not create thumbnails cache dir: {}", err);
      return LocalThumbnail::without_thumbnail(file_path, LocalThumbnailStatus::Failed, Some(file_hash));
    }
  };
  let thumbnail_jpg = thumbnails_dir.join(format!("{file_hash}.jpg"));
  let animated_webp = thumbnails_dir.join(format!("{file_hash}.webp"));

  // The filesystem is the source of truth; the DB flags are hints.
  if thumbnail_jpg.is_file() {
    return LocalThumbnail {
      file_path: file_path.to_string_lossy().into_owned(),
      status: LocalThumbnailStatus::Ready,
      maybe_file_hash: Some(file_hash),
      maybe_thumbnail_path: Some(thumbnail_jpg.to_string_lossy().into_owned()),
      maybe_animated_preview_path: animated_webp.is_file()
          .then(|| animated_webp.to_string_lossy().into_owned()),
    };
  }

  let extension = file_extension(file_path);
  let kind = if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
    MediaKind::Image
  } else if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
    MediaKind::Video
  } else {
    return LocalThumbnail::without_thumbnail(file_path, LocalThumbnailStatus::Unsupported, Some(file_hash));
  };

  let generation = {
    let _permit = GENERATION_SEMAPHORE.acquire().await.expect("semaphore never closes");
    let source = file_path.to_path_buf();
    let jpg = thumbnail_jpg.clone();
    let webp = animated_webp.clone();
    tokio::task::spawn_blocking(move || generate(kind, &source, &jpg, &webp)).await
  };

  match generation {
    Ok(Ok(wrote_animated_preview)) => {
      info!("Generated thumbnail for {} -> {}", file_path.display(), thumbnail_jpg.display());
      record_flags(database, file_path, true, wrote_animated_preview).await;
      LocalThumbnail {
        file_path: file_path.to_string_lossy().into_owned(),
        status: LocalThumbnailStatus::Ready,
        maybe_file_hash: Some(file_hash),
        maybe_thumbnail_path: Some(thumbnail_jpg.to_string_lossy().into_owned()),
        maybe_animated_preview_path: wrote_animated_preview
            .then(|| animated_webp.to_string_lossy().into_owned()),
      }
    }
    Ok(Err(err)) if err.is_unsupported() => {
      info!("Media not thumbnailable ({}): {}", file_path.display(), err);
      LocalThumbnail::without_thumbnail(file_path, LocalThumbnailStatus::Unsupported, Some(file_hash))
    }
    Ok(Err(err)) => {
      warn!("Thumbnail generation failed for {}: {}", file_path.display(), err);
      LocalThumbnail::without_thumbnail(file_path, LocalThumbnailStatus::Failed, Some(file_hash))
    }
    Err(join_error) => {
      warn!("Thumbnail generation task panicked for {}: {}", file_path.display(), join_error);
      LocalThumbnail::without_thumbnail(file_path, LocalThumbnailStatus::Failed, Some(file_hash))
    }
  }
}

// ── Private ──

#[derive(Copy, Clone, Debug)]
enum MediaKind {
  Image,
  Video,
}

/// Returns whether an animated preview was written alongside the static jpg.
fn generate(kind: MediaKind, source: &Path, jpg: &Path, webp: &Path) -> Result<bool, ThumbnailError> {
  match kind {
    MediaKind::Image => {
      generate_image_thumbnail(source, jpg, STATIC_THUMBNAIL_MAX_DIMENSION)?;
      Ok(false)
    }
    MediaKind::Video => {
      let outcome = generate_video_thumbnails(source, jpg, webp)?;
      Ok(outcome.wrote_animated_preview)
    }
  }
}

/// The memoization at the heart of this module: trust the indexed hash when
/// size+mtime match (a row lookup), rehash only when the file changed (a
/// full read).
async fn cached_or_computed_hash(
  database: &LocalFilesDatabase,
  file_path: &Path,
  file_size_bytes: i64,
  file_mtime_ms: i64,
) -> Result<String, String> {
  let path_string = file_path.to_string_lossy().into_owned();

  match get_local_file_by_path(database.get_connection(), &path_string).await {
    Ok(Some(record)) if record.matches_stat(file_size_bytes, file_mtime_ms) => {
      return Ok(record.file_hash_blake3);
    }
    Ok(_) => {}
    Err(err) => warn!("local_files lookup failed for {}: {}", path_string, err),
  }

  let hash_path = file_path.to_path_buf();
  let file_hash = tokio::task::spawn_blocking(move || hash_file_blake3(&hash_path))
      .await
      .map_err(|join_error| join_error.to_string())?
      .map_err(|io_error| io_error.to_string())?;

  let extension = file_extension(file_path);
  let upsert = upsert_local_file(UpsertLocalFileArgs {
    db: database.get_connection(),
    file_path: &path_string,
    maybe_file_type: (!extension.is_empty()).then_some(extension.as_str()),
    file_size_bytes,
    file_mtime_ms,
    file_hash_blake3: &file_hash,
  }).await;
  if let Err(err) = upsert {
    warn!("local_files upsert failed for {}: {}", path_string, err);
  }

  Ok(file_hash)
}

async fn record_flags(database: &LocalFilesDatabase, file_path: &Path, has_thumbnail: bool, has_animated_preview: bool) {
  let path_string = file_path.to_string_lossy();
  if let Err(err) = set_thumbnail_flags(database.get_connection(), &path_string, has_thumbnail, has_animated_preview).await {
    warn!("Could not record thumbnail flags for {}: {}", path_string, err);
  }
}

fn per_path_lock(file_path: &Path) -> Arc<TokioMutex<()>> {
  let mut locks = PER_PATH_LOCKS.lock().expect("lock poisoned");
  locks.entry(file_path.to_path_buf()).or_default().clone()
}

fn file_extension(file_path: &Path) -> String {
  file_path.extension()
      .and_then(|extension| extension.to_str())
      .map(str::to_ascii_lowercase)
      .unwrap_or_default()
}

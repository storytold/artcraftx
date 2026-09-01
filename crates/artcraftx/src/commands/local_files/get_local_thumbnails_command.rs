//! Bulk thumbnail lookup/generation for local files.
//!
//! Takes one or more absolute file paths and returns, per path, the cached
//! thumbnail (and video animated preview) — generating anything missing on
//! the fly. Per-item statuses mean one bad path never fails the batch, so
//! the command itself is infallible.

use std::path::PathBuf;

use serde_derive::Deserialize;

use crate::commands::utils::response::shorthand::InfallibleResponse;
use crate::commands::utils::response::success_response_wrapper::SerializeMarker;
use crate::services::local_files::thumbnail_service::{get_or_generate_thumbnail, LocalThumbnail};
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::database::local_files_database::LocalFilesDatabase;
use log::info;
use serde_derive::Serialize;
use tauri::State;

// ── Request ──

#[derive(Debug, Deserialize)]
pub struct GetLocalThumbnailsRequest {
  /// Absolute local file paths (typically completed results on disk).
  pub file_paths: Vec<String>,
}

// ── Response ──

#[derive(Debug, Serialize)]
pub struct GetLocalThumbnailsResponse {
  /// One entry per requested path, in request order.
  pub thumbnails: Vec<LocalThumbnail>,
}

impl SerializeMarker for GetLocalThumbnailsResponse {}

#[tauri::command]
pub async fn get_local_thumbnails_command(
  request: GetLocalThumbnailsRequest,
  app_data_root: State<'_, AppDataRoot>,
  local_files_database: State<'_, LocalFilesDatabase>,
) -> Result<InfallibleResponse<GetLocalThumbnailsResponse>, ()> {
  info!("get_local_thumbnails_command called for {} path(s)", request.file_paths.len());

  // Concurrent per path; actual CPU work is capped by the service's
  // generation semaphore and de-duplicated by its per-path locks.
  let file_paths: Vec<PathBuf> = request.file_paths.iter().map(PathBuf::from).collect();
  let lookups = file_paths.iter().map(|file_path| {
    get_or_generate_thumbnail(&local_files_database, &app_data_root, file_path)
  });
  let thumbnails = futures::future::join_all(lookups).await;

  Ok(GetLocalThumbnailsResponse { thumbnails }.into())
}

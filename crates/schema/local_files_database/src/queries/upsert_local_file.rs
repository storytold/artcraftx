use chrono::Utc;

use crate::connection::LocalFilesDbConnection;
use crate::error::LocalFilesDbError;

pub struct UpsertLocalFileArgs<'a> {
  pub db: &'a LocalFilesDbConnection,
  pub file_path: &'a str,
  pub maybe_file_type: Option<&'a str>,
  pub file_size_bytes: i64,
  pub file_mtime_ms: i64,
  pub file_hash_blake3: &'a str,
}

/// Insert or refresh the row for a path. A changed hash (file overwritten
/// in place) resets the thumbnail flags — the cache entries for the old
/// hash no longer describe this path's content.
pub async fn upsert_local_file(args: UpsertLocalFileArgs<'_>) -> Result<(), LocalFilesDbError> {
  let now = Utc::now();
  sqlx::query(
    r#"
    INSERT INTO local_files
      (file_path, file_type, file_size_bytes, file_mtime_ms, file_hash_blake3,
       has_thumbnail, has_animated_preview, created_at, updated_at)
    VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?6, ?6)
    ON CONFLICT(file_path) DO UPDATE SET
      file_type = excluded.file_type,
      file_size_bytes = excluded.file_size_bytes,
      file_mtime_ms = excluded.file_mtime_ms,
      file_hash_blake3 = excluded.file_hash_blake3,
      has_thumbnail = CASE
        WHEN local_files.file_hash_blake3 = excluded.file_hash_blake3
        THEN local_files.has_thumbnail ELSE 0 END,
      has_animated_preview = CASE
        WHEN local_files.file_hash_blake3 = excluded.file_hash_blake3
        THEN local_files.has_animated_preview ELSE 0 END,
      updated_at = excluded.updated_at
    "#,
  )
      .bind(args.file_path)
      .bind(args.maybe_file_type)
      .bind(args.file_size_bytes)
      .bind(args.file_mtime_ms)
      .bind(args.file_hash_blake3)
      .bind(now)
      .execute(args.db.get_pool())
      .await?;
  Ok(())
}

use chrono::{DateTime, Utc};

/// One row of `local_files`.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct LocalFileRecord {
  pub file_path: String,
  pub file_type: Option<String>,
  pub file_size_bytes: i64,
  pub file_mtime_ms: i64,
  pub file_hash_blake3: String,
  pub has_thumbnail: bool,
  pub has_animated_preview: bool,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

impl LocalFileRecord {
  /// Whether the cached hash is still current for a file with this stat.
  pub fn matches_stat(&self, file_size_bytes: i64, file_mtime_ms: i64) -> bool {
    self.file_size_bytes == file_size_bytes && self.file_mtime_ms == file_mtime_ms
  }
}

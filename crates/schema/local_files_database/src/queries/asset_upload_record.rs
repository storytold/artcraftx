use chrono::{DateTime, Utc};

/// One row of `asset_uploads`: a remote copy of some local content.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct AssetUploadRecord {
  pub service: String,
  pub account_id: String,
  pub file_hash_blake3: String,
  /// The service's own primary key for the asset.
  pub service_id: String,
  pub service_url: Option<String>,
  pub file_size_bytes: Option<i64>,
  pub uploaded_at: DateTime<Utc>,
  pub last_reused_at: Option<DateTime<Utc>>,
}

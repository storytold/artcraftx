use chrono::Utc;

use crate::connection::LocalFilesDbConnection;
use crate::error::LocalFilesDbError;

/// Record which cache artifacts exist for a path. Returns whether a row was
/// updated.
pub async fn set_thumbnail_flags(
  db: &LocalFilesDbConnection,
  file_path: &str,
  has_thumbnail: bool,
  has_animated_preview: bool,
) -> Result<bool, LocalFilesDbError> {
  let result = sqlx::query(
    r#"
    UPDATE local_files
    SET has_thumbnail = ?2, has_animated_preview = ?3, updated_at = ?4
    WHERE file_path = ?1
    "#,
  )
      .bind(file_path)
      .bind(has_thumbnail)
      .bind(has_animated_preview)
      .bind(Utc::now())
      .execute(db.get_pool())
      .await?;
  Ok(result.rows_affected() > 0)
}

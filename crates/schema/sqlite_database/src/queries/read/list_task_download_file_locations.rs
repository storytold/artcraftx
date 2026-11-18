use crate::connection::TaskDbConnection;
use crate::error::SqliteTasksError;

/// Every recorded first downloaded file (`on_complete_first_file_location`),
/// whatever the task's status. Lets the app check that a path it is asked to
/// open is a file one of its tasks downloaded.
///
/// NB: A runtime query on purpose, like `list_completed_task_download_files`:
/// it needs no compile-time schema and no `.sqlx` offline data.
pub async fn list_task_download_file_locations(
  db: &TaskDbConnection,
) -> Result<Vec<String>, SqliteTasksError> {
  let rows: Vec<(String,)> = sqlx::query_as(
    r#"
    SELECT on_complete_first_file_location
    FROM tasks
    WHERE on_complete_first_file_location IS NOT NULL
    "#,
  )
      .fetch_all(db.get_pool())
      .await?;
  Ok(rows.into_iter().map(|(path,)| path).collect())
}

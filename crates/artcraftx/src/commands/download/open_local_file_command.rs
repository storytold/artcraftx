use crate::commands::utils::response::shorthand::ResponseOrErrorMessage;
use crate::commands::utils::response::success_response_wrapper::SerializeMarker;
use crate::state::database::task_database::TaskDatabase;
use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use sqlite_database::queries::read::list_task_download_file_locations::list_task_download_file_locations;
use std::path::{Component, Path};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

/// How to open a local file.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenLocalFileMode {
  /// Open the file with the OS default application (image viewer, video player, ...).
  Open,
  /// Open the containing directory in the OS file manager (Finder, Explorer, ...)
  /// with the file selected.
  Reveal,
}

#[derive(Serialize)]
pub struct OpenLocalFileSuccessResponse {}

impl SerializeMarker for OpenLocalFileSuccessResponse {}

/// Open a file that a completed task downloaded (see the task's
/// `on_complete_first_file_location`) natively, either in its default viewer
/// or revealed in the file manager. The file must exist and must be a task's
/// recorded download.
#[tauri::command]
pub async fn open_local_file_command(
  app: AppHandle,
  task_database: State<'_, TaskDatabase>,
  path: String,
  mode: OpenLocalFileMode,
) -> ResponseOrErrorMessage<OpenLocalFileSuccessResponse> {
  info!("open_local_file_command called: mode={:?} path={:?}", mode, path);

  let file = Path::new(&path);

  if let Err(message) = check_openable(&task_database, file).await {
    return Err(message.into());
  }

  let result = match mode {
    OpenLocalFileMode::Open => app.opener().open_path(&path, None::<&str>),
    OpenLocalFileMode::Reveal => app.opener().reveal_item_in_dir(file),
  };

  if let Err(err) = result {
    error!("open_local_file_command failed ({:?}) for {:?}: {:?}", mode, path, err);
    return Err("could not open file".into());
  }

  Ok(OpenLocalFileSuccessResponse {}.into())
}

/// The file must exist and be a task's recorded download. Both paths are
/// canonicalized before they are compared.
async fn check_openable(task_database: &TaskDatabase, file: &Path) -> Result<(), &'static str> {
  if !file.is_file() {
    error!("open_local_file_command: not an existing file: {:?}", file);
    return Err("file not found");
  }

  let recorded = list_task_download_file_locations(task_database.get_connection())
      .await
      .map_err(|err| {
        error!("open_local_file_command: could not read task download locations: {:?}", err);
        "could not open file"
      })?;

  if !is_recorded_download(file, &recorded) {
    error!("open_local_file_command: not a recorded task download: {:?}", file);
    return Err("file is not a task download");
  }

  Ok(())
}

/// The request must name the file directly: a path with `..` components, or
/// a path that is itself a symlink, is refused even if it leads to a
/// recorded file.
fn is_recorded_download(file: &Path, recorded: &[String]) -> bool {
  if file.components().any(|component| component == Component::ParentDir) {
    return false;
  }
  let is_symlink = std::fs::symlink_metadata(file)
      .map(|metadata| metadata.file_type().is_symlink())
      .unwrap_or(true);
  if is_symlink {
    return false;
  }
  let Ok(file) = file.canonicalize() else {
    return false;
  };
  recorded.iter()
      .filter_map(|path| Path::new(path).canonicalize().ok())
      .any(|path| path == file)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::state::data_dir::app_data_root::AppDataRoot;
  use core_types::enums::generation_source::GenerationSource;
  use sqlite_database::queries::create::create_task::{create_task, CreateTaskArgs};
  use sqlite_database::queries::update::update_task_download_locations::{update_task_download_locations, UpdateTaskDownloadLocationsArgs};
  use sqlite_identifiers::enums::task_status::TaskStatus;
  use sqlite_identifiers::enums::task_type::TaskType;
  use std::path::PathBuf;
  use tempfile::TempDir;

  #[tokio::test]
  async fn recorded_download_is_accepted() {
    let (dir, task_database) = fresh_task_database().await;
    let recorded = write_file(&dir, "downloads/artcraft_image.png");
    record_download(&task_database, &recorded).await;

    assert_eq!(check_openable(&task_database, &recorded).await, Ok(()));
  }

  #[tokio::test]
  async fn unrecorded_existing_file_is_refused() {
    let (dir, task_database) = fresh_task_database().await;
    let recorded = write_file(&dir, "downloads/artcraft_image.png");
    record_download(&task_database, &recorded).await;
    let unrecorded = write_file(&dir, "elsewhere/notes.txt");

    assert_eq!(check_openable(&task_database, &unrecorded).await, Err("file is not a task download"));
  }

  #[tokio::test]
  async fn recorded_download_named_through_parent_dir_is_refused() {
    let (dir, task_database) = fresh_task_database().await;
    let recorded = write_file(&dir, "downloads/artcraft_image.png");
    record_download(&task_database, &recorded).await;
    std::fs::create_dir(dir.path().join("downloads/sub")).unwrap();
    let through_parent = dir.path().join("downloads/sub/../artcraft_image.png");

    assert_eq!(check_openable(&task_database, &through_parent).await, Err("file is not a task download"));
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn recorded_download_that_is_a_symlink_is_refused() {
    let (dir, task_database) = fresh_task_database().await;
    let elsewhere = write_file(&dir, "elsewhere/notes.txt");
    std::fs::create_dir_all(dir.path().join("downloads")).unwrap();
    let recorded = dir.path().join("downloads/artcraft_image.png");
    std::os::unix::fs::symlink(&elsewhere, &recorded).unwrap();
    record_download(&task_database, &recorded).await;

    assert_eq!(check_openable(&task_database, &recorded).await, Err("file is not a task download"));
  }

  async fn fresh_task_database() -> (TempDir, TaskDatabase) {
    let dir = tempfile::tempdir().unwrap();
    let root = AppDataRoot::create_existing(dir.path().join("root")).unwrap();
    let task_database = TaskDatabase::connect(&root).await.unwrap();
    (dir, task_database)
  }

  fn write_file(dir: &TempDir, relative_path: &str) -> PathBuf {
    let path = dir.path().join(relative_path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"contents").unwrap();
    path
  }

  async fn record_download(task_database: &TaskDatabase, file: &Path) {
    let db = task_database.get_connection();
    let task_id = create_task(CreateTaskArgs {
      db,
      status: TaskStatus::CompleteSuccess,
      task_type: TaskType::ImageGeneration,
      model_type: None,
      provider: GenerationSource::Artcraft,
      provider_job_id: None,
      is_batch_generation: false,
      queue_status_url: None,
      queue_response_url: None,
      prompt_token: None,
      frontend_caller: None,
      frontend_subscriber_id: None,
      frontend_subscriber_payload: None,
    }).await.unwrap();
    update_task_download_locations(UpdateTaskDownloadLocationsArgs {
      db,
      task_id: &task_id,
      directory_location: file.parent().unwrap(),
      first_file_location: file,
    }).await.unwrap();
  }
}

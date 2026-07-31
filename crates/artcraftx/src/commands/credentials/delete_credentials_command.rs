use crate::state::data_dir::app_data_root::AppDataRoot;
use log::{error, info};
use tauri::State;

/// Delete a credential file, identified by its file name within the
/// credentials directory (e.g. `fal_api_2.toml`). The frontend asks the
/// user for confirmation before calling this.
#[tauri::command]
pub async fn delete_credentials_command(
  app_data_root: State<'_, AppDataRoot>,
  file_name: String,
) -> Result<(), String> {
  info!("delete_credentials_command called for: {}", file_name);

  app_data_root
      .credentials_dir()
      .delete_credential_file(&file_name)
      .map_err(|err| {
        error!("Error deleting credential {}: {}", file_name, err);
        format!("Error deleting credential {}: {}", file_name, err)
      })
}

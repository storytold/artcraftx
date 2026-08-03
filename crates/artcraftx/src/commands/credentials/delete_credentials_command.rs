use crate::state::data_dir::app_data_root::AppDataRoot;
use log::{error, info};
use tauri::State;

/// Delete a credential, identified by its stable token. The frontend asks
/// the user for confirmation before calling this.
#[tauri::command]
pub async fn delete_credentials_command(
  app_data_root: State<'_, AppDataRoot>,
  credential_token: String,
) -> Result<(), String> {
  info!("delete_credentials_command called for: {}", credential_token);

  let creds_dir = app_data_root.credentials_dir();

  let credential = creds_dir
      .find_credential_by_token(&credential_token)
      .map_err(|err| {
        error!("Error looking up credential {}: {}", credential_token, err);
        format!("Error looking up credential {}: {}", credential_token, err)
      })?
      .ok_or_else(|| {
        let message = format!("No credential found for token {}", credential_token);
        error!("{}", message);
        message
      })?;

  creds_dir
      .delete_credential_file(&credential.id())
      .map_err(|err| {
        error!("Error deleting credential {}: {}", credential_token, err);
        format!("Error deleting credential {}: {}", credential_token, err)
      })
}

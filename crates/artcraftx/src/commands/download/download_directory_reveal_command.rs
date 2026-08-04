use crate::error::artcraftx_error::ArtcraftXError;
use crate::commands::response::shorthand::ResponseOrErrorMessage;
use crate::commands::response::success_response_wrapper::SerializeMarker;
use crate::state::app_preferences::app_preferences_manager::AppPreferencesManager;
use crate::state::data_dir::app_data_root::AppDataRoot;
use anyhow::anyhow;
use log::info;
use serde_derive::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize)]
pub struct DownloadDirectoryRevealSuccessResponse {
}

impl SerializeMarker for DownloadDirectoryRevealSuccessResponse {}

#[tauri::command]
pub async fn download_directory_reveal_command(
  app: AppHandle,
  app_prefs: State<'_, AppPreferencesManager>,
  app_data_root: State<'_, AppDataRoot>,
) -> ResponseOrErrorMessage<DownloadDirectoryRevealSuccessResponse> {

  info!("download_directory_reveal_command called");

  let result = handle_request(
    &app,
    &app_prefs,
    &app_data_root,
  ).await;

  if let Err(err) = result {
    format!("Error revealing download dir: {:?}", err);
    return Err("error revealing download dir".into())
  }

  Ok(DownloadDirectoryRevealSuccessResponse {}.into())
}


pub async fn handle_request(
  app: &AppHandle,
  app_prefs: &AppPreferencesManager,
  app_data_root: &AppDataRoot,
) -> Result<(), ArtcraftXError> {

  let app_prefs = app_prefs.get_clone()?;

  let download_directory = app_prefs
      .preferred_download_directory
      .download_directory(app_data_root);

  info!("Revealing item in directory: {:?}", download_directory);

  app.opener().reveal_item_in_dir(download_directory)
      .map_err(|err| anyhow!("Failed to open directory: {:?}", err))?;

  Ok(())
}

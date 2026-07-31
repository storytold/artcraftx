use crate::credentials::login_website::LoginWebsite;
use crate::login_window::open_login_window::open_login_window;
use crate::state::data_dir::app_data_root::AppDataRoot;
use log::{error, info};
use tauri::{AppHandle, State};

/// Open a web-login window for a site (e.g. the user clicked "Artcraft").
///
/// Opens a fresh, cookie-cleared webview and drives the login flow; a
/// background thread captures the resulting cookies into the credentials
/// directory once the user signs in.
#[tauri::command]
pub async fn open_web_login_command(
  app: AppHandle,
  app_data_root: State<'_, AppDataRoot>,
  website: LoginWebsite,
) -> Result<(), String> {
  info!("open_web_login_command called for website: {}", website);

  open_login_window(&app, &app_data_root, website)
      .await
      .map_err(|err| {
        error!("Error opening {} login window: {:?}", website, err);
        format!("Error opening {} login window: {}", website, err)
      })
}

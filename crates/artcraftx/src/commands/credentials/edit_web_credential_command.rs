use crate::commands::credentials::credential_payload::CredentialPayload;
use crate::credentials::cookie_credential::CookieCredential;
use crate::credentials::credential::CredentialSecret;
use crate::state::data_dir::app_data_root::AppDataRoot;
use chrono::Utc;
use log::{error, info};
use serde_derive::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct EditWebCredentialResponse {
  pub credential: CredentialPayload,
}

/// Edit an existing web-login (cookie) credential file, identified by its file
/// name within the credentials directory (e.g. `artcraft_cookies_2.toml`).
///
/// `name` replaces the label; an empty string clears it. `cookie_header`
/// replaces the cookies when provided (non-empty). Omitted fields are left
/// unchanged. The common case is simply renaming the credential.
#[tauri::command]
pub async fn edit_web_credential_command(
  app_data_root: State<'_, AppDataRoot>,
  file_name: String,
  name: Option<String>,
  cookie_header: Option<String>,
) -> Result<EditWebCredentialResponse, String> {
  info!("edit_web_credential_command called for: {}", file_name);

  let creds_dir = app_data_root.credentials_dir();

  let mut credential = creds_dir
      .load_credential(&file_name)
      .map_err(|err| {
        error!("Error loading credential {}: {}", file_name, err);
        format!("Error loading credential {}: {}", file_name, err)
      })?;

  let CredentialSecret::Cookies(existing_cookie) = &credential.secret else {
    let message = format!("Credential {} is not a cookie credential", file_name);
    error!("{}", message);
    return Err(message);
  };
  let existing_cookie_header = existing_cookie.cookie_header.clone();
  let existing_failed_at = existing_cookie.failed_at;
  let existing_succeeded_at = existing_cookie.succeeded_at;

  if let Some(new_cookie_header) = cookie_header {
    let new_cookie_header = new_cookie_header.trim().to_string();
    if !new_cookie_header.is_empty() && new_cookie_header != existing_cookie_header {
      credential.secret = CredentialSecret::Cookies(CookieCredential {
        cookie_header: new_cookie_header,
        updated_at: Some(Utc::now()),
        failed_at: existing_failed_at,
        succeeded_at: existing_succeeded_at,
      });
    }
  }

  if let Some(new_name) = name {
    let new_name = new_name.trim().to_string();
    credential.name = if new_name.is_empty() { None } else { Some(new_name) };
  }

  creds_dir
      .save_credential(&credential)
      .map_err(|err| {
        error!("Error saving credential {}: {}", file_name, err);
        format!("Error saving credential {}: {}", file_name, err)
      })?;

  Ok(EditWebCredentialResponse {
    credential: CredentialPayload::from_credential(&credential),
  })
}

use crate::commands::credentials::credential_payload::CredentialPayload;
use crate::credentials::cookie_credential::CookieCredential;
use crate::credentials::auth_credential::{AuthCredential, CredentialSecret};
use core_types::enums::generation_source::{CredentialKind, GenerationSource};
use crate::credentials::credential_user_info::CredentialUserInfo;
use crate::credentials::service_cookie_origin::cookie_origin_for_service;
use crate::error::artcraftx_error::ArtcraftXError;
use cookie_store_wrapper::cookie_store::CookieStore;
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::data_dir::subdirectory::app_credentials_dir::AppCredentialsDir;
use chrono::Utc;
use log::{error, info};
use serde_derive::Serialize;
use tauri::State;

/// The captured result of a web login, ready to persist.
pub struct WebCredentialSave {
  pub service: GenerationSource,
  pub cookies: CookieStore,
  pub maybe_user_info: Option<CredentialUserInfo>,
}

/// Save a web-login (cookie) credential to the credentials directory.
///
/// Upserts by service: if a managed cookie credential for `service` already
/// exists, its file (and label) are reused and the cookies refreshed;
/// otherwise a new `{service}.toml` file is created. This keeps the login flow
/// from piling up a new file on every re-login while still letting users
/// hand-maintain multiple accounts.
pub fn save_web_credential(
  creds_dir: &AppCredentialsDir,
  save: WebCredentialSave,
) -> Result<AuthCredential, ArtcraftXError> {
  let existing = creds_dir
      .load_credentials()?
      .into_iter()
      .find(|credential| {
        credential.service == save.service && credential.kind() == CredentialKind::Cookies
      });

  let cookie = CookieCredential {
    updated_at: Some(Utc::now()),
    failed_at: None,
    succeeded_at: None,
    cookies: save.cookies,
  };

  let credential = match existing {
    Some(existing) => AuthCredential {
      id: existing.id,
      service: existing.service,
      name: existing.name,
      secret: CredentialSecret::Cookies(cookie),
      user_info: merge_user_info(existing.user_info, save.maybe_user_info),
      source_path: existing.source_path,
    },
    None => AuthCredential {
      id: creds_dir.generate_unique_credential_id(),
      service: save.service,
      name: None,
      secret: CredentialSecret::Cookies(cookie),
      user_info: save.maybe_user_info,
      source_path: creds_dir.next_available_credential_path(save.service),
    },
  };

  creds_dir.save_credential(&credential)?;
  Ok(credential)
}

/// Prefer freshly detected identity fields, falling back to whatever the
/// existing file already recorded, so a login that can't read (say) an email
/// doesn't wipe one the user previously had.
fn merge_user_info(
  existing: Option<CredentialUserInfo>,
  fresh: Option<CredentialUserInfo>,
) -> Option<CredentialUserInfo> {
  match (existing, fresh) {
    (None, fresh) => fresh,
    (existing, None) => existing,
    (Some(existing), Some(fresh)) => Some(CredentialUserInfo {
      username: fresh.username.or(existing.username),
      email: fresh.email.or(existing.email),
    }),
  }
}

#[derive(Serialize)]
pub struct AddWebCredentialResponse {
  pub credential: CredentialPayload,
}

/// Manually add a web-login (cookie) credential file for a service, given a
/// full `Cookie:` header. The automated login flow saves via
/// [`save_web_credential`]; this command is the hand-entry path.
#[tauri::command]
pub async fn add_web_credential_command(
  app_data_root: State<'_, AppDataRoot>,
  service: GenerationSource,
  cookie_header: String,
  name: Option<String>,
) -> Result<AddWebCredentialResponse, String> {
  info!("add_web_credential_command called for service: {}", service);

  if service.kind() != CredentialKind::Cookies {
    let message = format!("Service {} is not a cookie service", service);
    error!("{}", message);
    return Err(message);
  }

  let cookie_header = cookie_header.trim().to_string();
  if cookie_header.is_empty() {
    return Err("Cookie header must not be empty".to_string());
  }

  let Some(cookie_origin) = cookie_origin_for_service(service) else {
    let message = format!("Service {} has no cookie origin", service);
    error!("{}", message);
    return Err(message);
  };
  let cookies = CookieStore::from_cookie_header(&cookie_header, &cookie_origin);
  if cookies.is_empty() {
    return Err("Cookie header contained no name=value cookies".to_string());
  }

  let credential = save_web_credential(
    app_data_root.credentials_dir(),
    WebCredentialSave {
      service,
      cookies,
      maybe_user_info: None,
    },
  ).map_err(|err| {
    error!("Error saving web credential: {}", err);
    format!("Error saving web credential: {}", err)
  })?;

  let credential = apply_optional_name(app_data_root.credentials_dir(), credential, name)?;

  Ok(AddWebCredentialResponse {
    credential: CredentialPayload::from_auth_credential(&credential),
  })
}

/// Apply an optional user-supplied label to a just-saved credential.
fn apply_optional_name(
  creds_dir: &AppCredentialsDir,
  mut credential: AuthCredential,
  name: Option<String>,
) -> Result<AuthCredential, String> {
  let Some(name) = name else {
    return Ok(credential);
  };
  let name = name.trim().to_string();
  credential.name = if name.is_empty() { None } else { Some(name) };
  creds_dir.save_credential(&credential).map_err(|err| {
    error!("Error saving credential label: {}", err);
    format!("Error saving credential label: {}", err)
  })?;
  Ok(credential)
}

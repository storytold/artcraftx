use artcraft_client::api_defs::users::login::{LoginErrorType, LoginRequest};
use artcraft_client::endpoints::users::login::{login, LoginArgs, LoginError};
use artcraft_client::utils::api_host::ApiHost;
use chrono::Utc;
use enums::common::generation_provider::GenerationProvider;
use log::{error, info, warn};
use serde_derive::Serialize;
use tauri::{AppHandle, State};

use crate::commands::credentials::credential_payload::CredentialPayload;
use crate::credentials::cookie_credential::CookieCredential;
use crate::credentials::credential::{Credential, CredentialSecret};
use crate::credentials::credential_service_type::CredentialServiceType;
use crate::credentials::credential_user_info::CredentialUserInfo;
use crate::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::events::functional_events::refresh_account_state_event::RefreshAccountStateEvent;
use crate::state::data_dir::app_data_root::AppDataRoot;

/// Port the local ArtCraft development server listens on.
const LOCAL_DEV_PORT: u32 = 12345;

#[derive(Serialize)]
pub struct ArtcraftLoginResponse {
  pub credential: CredentialPayload,
}

/// Error payload surfaced to the frontend when a login fails.
#[derive(Clone, Debug, Serialize)]
pub struct ArtcraftLoginCommandError {
  pub error_type: ArtcraftLoginErrorType,
  pub message: String,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum ArtcraftLoginErrorType {
  /// Invalid username/email or password.
  #[serde(rename = "invalid_credentials")]
  InvalidCredentials,
  /// The account was created without a password (needs a password reset).
  #[serde(rename = "account_needs_password")]
  AccountNeedsPassword,
  /// The API returned a server error.
  #[serde(rename = "server_error")]
  ServerError,
  /// We couldn't reach the API (network problem, local dev server not
  /// running, unexpected response).
  #[serde(rename = "connection_error")]
  ConnectionError,
  /// The request was malformed (wrong service, empty fields).
  #[serde(rename = "bad_request")]
  BadRequest,
}

/// Log into an ArtCraft account with a username (or email) and password, then
/// store the resulting session as a new credential file.
///
/// `service` selects the environment: `Artcraft` hits production
/// (`api.storyteller.ai`); `ArtcraftLocal` hits a local dev server. Every
/// successful login creates a NEW credential file, so users can keep an
/// unlimited number of accounts for either environment.
#[tauri::command]
pub async fn artcraft_login_command(
  app: AppHandle,
  app_data_root: State<'_, AppDataRoot>,
  service: CredentialServiceType,
  username_or_email: String,
  password: String,
) -> Result<ArtcraftLoginResponse, ArtcraftLoginCommandError> {
  info!("artcraft_login_command called for service: {}", service);

  let api_host = match service {
    CredentialServiceType::Artcraft => ApiHost::Storyteller,
    CredentialServiceType::ArtcraftLocal => ApiHost::Localhost { port: LOCAL_DEV_PORT },
    other => {
      let message = format!("Service {} does not support password login", other);
      error!("{}", message);
      return Err(ArtcraftLoginCommandError {
        error_type: ArtcraftLoginErrorType::BadRequest,
        message,
      });
    }
  };

  let username_or_email = username_or_email.trim().to_string();
  if username_or_email.is_empty() || password.is_empty() {
    return Err(ArtcraftLoginCommandError {
      error_type: ArtcraftLoginErrorType::BadRequest,
      message: "Enter a username (or email) and password".to_string(),
    });
  }

  let request = LoginRequest {
    username_or_email: username_or_email.clone(),
    password,
  };

  let response = login(LoginArgs {
    api_host: &api_host,
    request: &request,
  }).await.map_err(|err| {
    warn!("ArtCraft login failed for service {}: {}", service, err);
    login_error_to_command_error(err)
  })?;

  let credential = save_session_credential(
    &app_data_root,
    service,
    &response.signed_session,
    username_or_email,
  )?;

  RefreshAccountStateEvent {
    provider: Some(GenerationProvider::Artcraft),
  }.send_infallible(&app);

  Ok(ArtcraftLoginResponse {
    credential: CredentialPayload::from_credential(&credential),
  })
}

/// Persist the signed session as a NEW cookie credential file (never upserts,
/// so each login adds another account).
fn save_session_credential(
  app_data_root: &AppDataRoot,
  service: CredentialServiceType,
  signed_session: &str,
  username_or_email: String,
) -> Result<Credential, ArtcraftLoginCommandError> {
  let now = Utc::now();

  let cookie = CookieCredential {
    cookie_header: format!("session={}", signed_session),
    updated_at: Some(now),
    failed_at: None,
    succeeded_at: Some(now),
  };

  let user_info = if username_or_email.contains('@') {
    CredentialUserInfo {
      username: None,
      email: Some(username_or_email),
    }
  } else {
    CredentialUserInfo {
      username: Some(username_or_email),
      email: None,
    }
  };

  let creds_dir = app_data_root.credentials_dir();

  let credential = Credential {
    token: creds_dir.generate_unique_credential_token(),
    service,
    name: None,
    secret: CredentialSecret::Cookies(cookie),
    user_info: Some(user_info),
    source_path: creds_dir.next_available_credential_path(service),
  };

  creds_dir.save_credential(&credential).map_err(|err| {
    error!("Error saving login credential: {}", err);
    ArtcraftLoginCommandError {
      error_type: ArtcraftLoginErrorType::ServerError,
      message: format!("Login succeeded but saving the credential failed: {}", err),
    }
  })?;

  Ok(credential)
}

fn login_error_to_command_error(error: LoginError) -> ArtcraftLoginCommandError {
  match error {
    LoginError::Login(response) => {
      let error_type = match response.error_type {
        LoginErrorType::InvalidCredentials => ArtcraftLoginErrorType::InvalidCredentials,
        LoginErrorType::AccountNeedsPassword => ArtcraftLoginErrorType::AccountNeedsPassword,
        LoginErrorType::ServerError => ArtcraftLoginErrorType::ServerError,
      };
      ArtcraftLoginCommandError {
        error_type,
        message: response.error_message,
      }
    }
    LoginError::Storyteller(err) => ArtcraftLoginCommandError {
      error_type: ArtcraftLoginErrorType::ConnectionError,
      message: format!("Could not reach the server: {}", err),
    },
  }
}

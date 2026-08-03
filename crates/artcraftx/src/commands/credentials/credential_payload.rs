use crate::credentials::credential::Credential;
use crate::credentials::credential_service_type::{CredentialKind, CredentialServiceType};
use chrono::{DateTime, Utc};
use serde_derive::Serialize;
use tokens::tokens::sqlite::credentials::CredentialToken;

/// Frontend-facing view of a stored credential.
#[derive(Serialize)]
pub struct CredentialPayload {
  /// The credential's stable identity (`credential_{entropy}`). Hidden from
  /// users, but the effective primary identifier — pass this back to
  /// edit/delete and use it to reference the credential anywhere.
  pub token: CredentialToken,
  /// The credential's file name within the credentials directory
  /// (e.g. `fal_api_2.toml`). Informational; may change if the user renames
  /// the file.
  pub id: String,
  pub service: CredentialServiceType,
  pub kind: CredentialKind,
  /// Optional user-facing label. Empty by default.
  pub name: Option<String>,
  /// Full API key (local app; the user owns this file).
  pub api_key: Option<String>,
  /// First few characters of the API key, for display.
  pub api_key_preview: Option<String>,
  pub username: Option<String>,
  pub email: Option<String>,
  pub updated_at: Option<DateTime<Utc>>,
  pub failed_at: Option<DateTime<Utc>>,
  pub succeeded_at: Option<DateTime<Utc>>,
}

impl CredentialPayload {
  pub fn from_credential(credential: &Credential) -> Self {
    let api_key = credential.api_key();
    let cookie = credential.cookies();
    Self {
      token: credential.token.clone(),
      id: credential.id(),
      service: credential.service,
      kind: credential.kind(),
      name: credential.name.clone(),
      api_key: api_key.map(|key| key.api_key.clone()),
      api_key_preview: api_key.map(|key| key.printable_partial_prefix.clone()),
      username: credential.user_info.as_ref().and_then(|info| info.username.clone()),
      email: credential.user_info.as_ref().and_then(|info| info.email.clone()),
      updated_at: cookie.and_then(|c| c.updated_at),
      failed_at: api_key.and_then(|key| key.failed_at).or(cookie.and_then(|c| c.failed_at)),
      succeeded_at: api_key.and_then(|key| key.succeeded_at).or(cookie.and_then(|c| c.succeeded_at)),
    }
  }
}

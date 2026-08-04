use crate::credentials::api_key_credential::ApiKeyCredential;
use crate::credentials::cookie_credential::CookieCredential;
use crate::credentials::credential_file::CredentialFile;
use crate::credentials::credential_service_type::{CredentialKind, CredentialServiceType};
use crate::credentials::credential_user_info::CredentialUserInfo;
use crate::error::artcraftx_credential_error::ArtcraftXCredentialError;
use std::path::{Path, PathBuf};
use identifiers::CredentialId;

/// A validated, in-app credential.
///
/// Unlike the on-disk [`CredentialFile`], a `Credential` always carries
/// exactly one secret and remembers which file it was loaded from, so the
/// app can rewrite that file in place (refreshed cookies, success/failure
/// timestamps, etc.)
#[derive(Clone, Debug)]
pub struct Credential {
  /// The credential's stable identity (`credential_{entropy}`), stored in the
  /// TOML file. Hidden from users, but the effective primary identifier —
  /// file names can be freely renamed.
  pub id: CredentialId,
  pub service: CredentialServiceType,
  /// Optional user-facing label to tell multiple credentials for the same
  /// service apart. Empty by default.
  pub name: Option<String>,
  pub secret: CredentialSecret,
  pub user_info: Option<CredentialUserInfo>,
  /// The TOML file this credential was loaded from (and saves back to).
  pub source_path: PathBuf,
}

/// The secret payload of a credential. Mutually exclusive by construction.
#[derive(Clone, Debug)]
pub enum CredentialSecret {
  Cookies(CookieCredential),
  ApiKey(ApiKeyCredential),
}

impl CredentialSecret {
  pub fn kind(&self) -> CredentialKind {
    match self {
      Self::Cookies(_) => CredentialKind::Cookies,
      Self::ApiKey(_) => CredentialKind::ApiKey,
    }
  }
}

impl Credential {
  /// Load a credential, minting (and persisting) an id if the file lacks
  /// one so the identity stays stable across loads.
  pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ArtcraftXCredentialError> {
    let path = path.as_ref();
    let file = CredentialFile::load(path)?;
    let needs_id_backfill = file.id.is_none();
    let credential = file.into_credential(path.to_path_buf())?;
    if needs_id_backfill {
      if let Err(err) = credential.save() {
        log::warn!("Could not persist backfilled credential id for {:?}: {}", path, err);
      }
    }
    Ok(credential)
  }

  /// Rewrite this credential's TOML file in place.
  pub fn save(&self) -> Result<(), ArtcraftXCredentialError> {
    CredentialFile::from_credential(self).save(&self.source_path)
  }

  pub fn kind(&self) -> CredentialKind {
    self.secret.kind()
  }

  /// The credential's file name within the credentials directory
  /// (e.g. `fal_api_2.toml`). Informational — the stable identifier is
  /// [`Credential::id`]; users can rename files freely.
  pub fn file_name(&self) -> String {
    self.source_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
  }

  pub fn cookies(&self) -> Option<&CookieCredential> {
    match &self.secret {
      CredentialSecret::Cookies(cookie) => Some(cookie),
      CredentialSecret::ApiKey(_) => None,
    }
  }

  pub fn api_key(&self) -> Option<&ApiKeyCredential> {
    match &self.secret {
      CredentialSecret::ApiKey(api_key) => Some(api_key),
      CredentialSecret::Cookies(_) => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn save_and_load_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fal_api_key.toml");

    let credential = Credential {
      id: CredentialId::generate(),
      service: CredentialServiceType::FalApi,
      name: Some("work account".to_string()),
      secret: CredentialSecret::ApiKey(ApiKeyCredential::new("fal-secret-key-12345")),
      user_info: None,
      source_path: path.clone(),
    };
    credential.save().unwrap();

    let loaded = Credential::load_from_file(&path).unwrap();
    assert_eq!(loaded.id, credential.id);
    assert_eq!(loaded.service, CredentialServiceType::FalApi);
    assert_eq!(loaded.name.as_deref(), Some("work account"));
    assert_eq!(loaded.api_key().unwrap().api_key, "fal-secret-key-12345");
    assert_eq!(loaded.api_key().unwrap().printable_partial_prefix, "fal-s");
    assert_eq!(loaded.source_path, path);
    assert_eq!(loaded.file_name(), "fal_api_key.toml");
  }
}

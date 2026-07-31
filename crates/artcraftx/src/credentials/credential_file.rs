use crate::credentials::api_key_credential::ApiKeyCredential;
use crate::credentials::cookie_credential::CookieCredential;
use crate::credentials::credential::{Credential, CredentialSecret};
use crate::credentials::credential_service_type::CredentialServiceType;
use crate::credentials::credential_user_info::CredentialUserInfo;
use crate::error::artcraftx_credential_error::ArtcraftXCredentialError;
use serde_derive::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The on-disk (TOML) shape of a credential file.
///
/// This is the tolerant serialization layer: both secrets are optional so
/// that hand-written files always parse, and validation happens when
/// converting into the in-app [`Credential`] type.
///
/// Example file (`~/Artcraft/artcraftx/credentials/higgsfield.toml`):
///
/// ```toml
/// service = "higgsfield_cookies"
///
/// [cookie]
/// cookie_header = "session=abc123; other=value"
///
/// [user_info]
/// username = "creator123"
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CredentialFile {
  pub service: CredentialServiceType,

  /// Optional user-facing label. Empty by default.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,

  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub cookie: Option<CookieCredential>,

  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub api_key: Option<ApiKeyCredential>,

  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub user_info: Option<CredentialUserInfo>,
}

impl CredentialFile {
  pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ArtcraftXCredentialError> {
    let path = path.as_ref();
    let contents = std::fs::read_to_string(path)
        .map_err(|source| ArtcraftXCredentialError::FileReadError {
          path: path.to_path_buf(),
          source,
        })?;
    toml::from_str(&contents)
        .map_err(|source| ArtcraftXCredentialError::TomlParseError {
          path: path.to_path_buf(),
          source,
        })
  }

  pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ArtcraftXCredentialError> {
    let path = path.as_ref();
    let contents = toml::to_string_pretty(self)
        .map_err(|source| ArtcraftXCredentialError::TomlSerializeError { source })?;
    std::fs::write(path, contents)
        .map_err(|source| ArtcraftXCredentialError::FileWriteError {
          path: path.to_path_buf(),
          source,
        })
  }

  /// Validate and convert into the in-app credential type.
  ///
  /// Exactly one of `cookie` / `api_key` must be present, and it must match
  /// the auth mechanism of `service`.
  pub fn into_credential(self, source_path: PathBuf) -> Result<Credential, ArtcraftXCredentialError> {
    let secret = match (self.cookie, self.api_key) {
      (Some(cookie), None) => CredentialSecret::Cookies(cookie),
      (None, Some(mut api_key)) => {
        api_key.normalize();
        CredentialSecret::ApiKey(api_key)
      },
      (None, None) => {
        return Err(ArtcraftXCredentialError::MissingSecret { path: source_path });
      },
      (Some(_), Some(_)) => {
        return Err(ArtcraftXCredentialError::AmbiguousSecret { path: source_path });
      },
    };

    if secret.kind() != self.service.kind() {
      return Err(ArtcraftXCredentialError::SecretKindMismatch {
        path: source_path,
        service: self.service,
        found: secret.kind(),
      });
    }

    Ok(Credential {
      service: self.service,
      name: self.name,
      secret,
      user_info: self.user_info,
      source_path,
    })
  }

  pub fn from_credential(credential: &Credential) -> Self {
    let (cookie, api_key) = match &credential.secret {
      CredentialSecret::Cookies(cookie) => (Some(cookie.clone()), None),
      CredentialSecret::ApiKey(api_key) => (None, Some(api_key.clone())),
    };
    Self {
      service: credential.service,
      name: credential.name.clone(),
      cookie,
      api_key,
      user_info: credential.user_info.clone(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::credentials::credential_service_type::CredentialKind;

  const HAND_WRITTEN_COOKIE_FILE: &str = r#"
    service = "artcraft_cookies"

    [cookie]
    cookie_header = "session=abc123"

    [user_info]
    username = "creator123"
    email = "creator@example.com"
  "#;

  const HAND_WRITTEN_API_KEY_FILE: &str = r#"
    service = "fal_api"

    [api_key]
    api_key = "fal-secret-key-12345"
  "#;

  mod parsing_tests {
    use super::*;

    #[test]
    fn parses_hand_written_cookie_file() {
      let file: CredentialFile = toml::from_str(HAND_WRITTEN_COOKIE_FILE).unwrap();
      assert_eq!(file.service, CredentialServiceType::ArtcraftCookies);
      assert_eq!(file.cookie.unwrap().cookie_header, "session=abc123");
      let user_info = file.user_info.unwrap();
      assert_eq!(user_info.username.as_deref(), Some("creator123"));
      assert_eq!(user_info.email.as_deref(), Some("creator@example.com"));
    }

    #[test]
    fn parses_hand_written_api_key_file_without_prefix() {
      let file: CredentialFile = toml::from_str(HAND_WRITTEN_API_KEY_FILE).unwrap();
      assert_eq!(file.service, CredentialServiceType::FalApi);
      assert_eq!(file.api_key.unwrap().api_key, "fal-secret-key-12345");
    }

    #[test]
    fn rejects_unknown_service() {
      let result: Result<CredentialFile, _> =
          toml::from_str("service = \"unknown_service\"\n[api_key]\napi_key = \"x\"\n");
      assert!(result.is_err());
    }
  }

  mod validation_tests {
    use super::*;

    #[test]
    fn cookie_file_becomes_cookie_credential() {
      let file: CredentialFile = toml::from_str(HAND_WRITTEN_COOKIE_FILE).unwrap();
      let credential = file.into_credential(PathBuf::from("test.toml")).unwrap();
      assert_eq!(credential.kind(), CredentialKind::Cookies);
      assert_eq!(credential.source_path, PathBuf::from("test.toml"));
    }

    #[test]
    fn api_key_prefix_is_computed_on_load() {
      let file: CredentialFile = toml::from_str(HAND_WRITTEN_API_KEY_FILE).unwrap();
      let credential = file.into_credential(PathBuf::from("test.toml")).unwrap();
      let CredentialSecret::ApiKey(api_key) = &credential.secret else {
        panic!("expected api key secret");
      };
      assert_eq!(api_key.printable_partial_prefix, "fal-s");
    }

    #[test]
    fn missing_secret_is_an_error() {
      let file: CredentialFile = toml::from_str("service = \"fal_api\"").unwrap();
      let result = file.into_credential(PathBuf::from("test.toml"));
      assert!(matches!(result, Err(ArtcraftXCredentialError::MissingSecret { .. })));
    }

    #[test]
    fn both_secrets_is_an_error() {
      let toml_text = r#"
        service = "fal_api"
        [cookie]
        cookie_header = "a=b"
        [api_key]
        api_key = "x"
      "#;
      let file: CredentialFile = toml::from_str(toml_text).unwrap();
      let result = file.into_credential(PathBuf::from("test.toml"));
      assert!(matches!(result, Err(ArtcraftXCredentialError::AmbiguousSecret { .. })));
    }

    #[test]
    fn secret_kind_must_match_service_kind() {
      let toml_text = r#"
        service = "fal_api"
        [cookie]
        cookie_header = "a=b"
      "#;
      let file: CredentialFile = toml::from_str(toml_text).unwrap();
      let result = file.into_credential(PathBuf::from("test.toml"));
      assert!(matches!(result, Err(ArtcraftXCredentialError::SecretKindMismatch { .. })));
    }
  }

  mod round_trip_tests {
    use super::*;

    #[test]
    fn credential_round_trips_through_toml() {
      let file: CredentialFile = toml::from_str(HAND_WRITTEN_COOKIE_FILE).unwrap();
      let credential = file.into_credential(PathBuf::from("test.toml")).unwrap();
      let rewritten = CredentialFile::from_credential(&credential);
      let toml_text = toml::to_string_pretty(&rewritten).unwrap();
      let reparsed: CredentialFile = toml::from_str(&toml_text).unwrap();
      assert_eq!(reparsed, rewritten);
    }
  }
}

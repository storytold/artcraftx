use crate::credentials::credential::Credential;
use crate::error::artcraftx_credential_error::ArtcraftXCredentialError;
use crate::error::artcraftx_error::ArtcraftXError;
use crate::state::data_dir::subdirectory::trait_data_subdir::DataSubdir;
use std::path::{Path, PathBuf};

/// The directory holding per-service credential TOML files
/// (by default `~/Artcraft/artcraftx/credentials`).
///
/// Files can be named anything (`artcraft_user1.toml`, `fal_api_key.toml`,
/// `higgsfield.toml`, ...) — users may hand-write their own. See
/// [`crate::credentials`] for the file format.
#[derive(Clone)]
pub struct AppCredentialsDir {
  path: PathBuf,
}

impl DataSubdir for AppCredentialsDir {
  const DIRECTORY_NAME: &'static str = "credentials";

  fn new_from<P: AsRef<Path>> (dir: P) -> Self {
    Self {
      path: dir.as_ref().to_path_buf(),
    }
  }

  fn path(&self) -> &Path {
    &self.path
  }
}

impl AppCredentialsDir {
  /// List and load every `*.toml` credential file in the directory.
  ///
  /// Files that fail to parse or validate are skipped with a warning so a
  /// single malformed (possibly hand-written) file can't take down every
  /// other credential. Only a directory listing failure is an error.
  pub fn load_credentials(&self) -> Result<Vec<Credential>, ArtcraftXError> {
    let entries = std::fs::read_dir(&self.path)
        .map_err(|source| ArtcraftXCredentialError::DirectoryReadError {
          path: self.path.clone(),
          source,
        })?;

    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && has_toml_extension(path))
        .collect();
    paths.sort();

    let mut credentials = Vec::new();
    for path in paths {
      match Credential::load_from_file(&path) {
        Ok(credential) => credentials.push(credential),
        Err(err) => {
          log::warn!("Skipping bad credential file: {}", err);
        },
      }
    }

    Ok(credentials)
  }

  /// Rewrite a credential's TOML file in place (refreshed cookies,
  /// success/failure timestamps, etc.)
  pub fn save_credential(&self, credential: &Credential) -> Result<(), ArtcraftXError> {
    credential.save().map_err(ArtcraftXError::from)
  }

  /// Path for a new managed credential file with the given file stem,
  /// e.g. `file_path_for("fal_api_key")` -> `.../credentials/fal_api_key.toml`.
  pub fn file_path_for(&self, file_stem: &str) -> PathBuf {
    self.path.join(format!("{}.toml", file_stem))
  }
}

fn has_toml_extension(path: &Path) -> bool {
  path.extension()
      .map(|ext| ext.eq_ignore_ascii_case("toml"))
      .unwrap_or(false)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::credentials::api_key_credential::ApiKeyCredential;
  use crate::credentials::credential::CredentialSecret;
  use crate::credentials::credential_service_type::CredentialServiceType;

  fn write_file(dir: &Path, name: &str, contents: &str) {
    std::fs::write(dir.join(name), contents).unwrap();
  }

  #[test]
  fn loads_all_valid_toml_files() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
      dir.path(),
      "artcraft_user1.toml",
      "service = \"artcraft_cookies\"\n[cookie]\ncookie_header = \"a=b\"\n",
    );
    write_file(
      dir.path(),
      "fal_api_key.toml",
      "service = \"fal_api\"\n[api_key]\napi_key = \"fal-key-123\"\n",
    );
    // Non-toml and malformed files must not break the load.
    write_file(dir.path(), "notes.txt", "not a credential");
    write_file(dir.path(), "broken.toml", "service = \"fal_api\"");

    let creds_dir = AppCredentialsDir::new_from(dir.path());
    let credentials = creds_dir.load_credentials().unwrap();

    assert_eq!(credentials.len(), 2);
    assert_eq!(credentials[0].service, CredentialServiceType::ArtcraftCookies);
    assert_eq!(credentials[1].service, CredentialServiceType::FalApi);
  }

  #[test]
  fn save_credential_writes_to_source_path() {
    let dir = tempfile::tempdir().unwrap();
    let creds_dir = AppCredentialsDir::new_from(dir.path());

    let credential = Credential {
      service: CredentialServiceType::FalApi,
      secret: CredentialSecret::ApiKey(ApiKeyCredential::new("fal-key-123")),
      user_info: None,
      source_path: creds_dir.file_path_for("fal_api_key"),
    };
    creds_dir.save_credential(&credential).unwrap();

    let credentials = creds_dir.load_credentials().unwrap();
    assert_eq!(credentials.len(), 1);
    assert_eq!(credentials[0].api_key().unwrap().api_key, "fal-key-123");
  }
}

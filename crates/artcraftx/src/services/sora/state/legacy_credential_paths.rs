//! Legacy on-disk credential paths for Sora.
//!
//! Implemented for both the credentials dir and the app data root because
//! legacy call sites use both receivers.
//!
//! TODO(artcraftx): retire once Sora moves to the unified TOML credential
//! store in `crate::credentials`.

use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::data_dir::subdirectory::app_credentials_dir::AppCredentialsDir;
use crate::state::data_dir::subdirectory::trait_data_subdir::DataSubdir;
use std::path::PathBuf;

pub trait SoraLegacyCredentialPaths {
  fn get_sora_cookie_file_path(&self) -> PathBuf;
  fn get_sora_bearer_token_file_path(&self) -> PathBuf;
  fn get_sora_legacy_sentinel_file_path(&self) -> PathBuf;
  fn get_sora_sentinel_token_file_path(&self) -> PathBuf;
}

impl SoraLegacyCredentialPaths for AppCredentialsDir {
  fn get_sora_cookie_file_path(&self) -> PathBuf {
    self.path().join("sora_cookies.txt")
  }

  fn get_sora_bearer_token_file_path(&self) -> PathBuf {
    self.path().join("sora_bearer_token.txt")
  }

  fn get_sora_legacy_sentinel_file_path(&self) -> PathBuf {
    self.path().join("sora_sentinel.txt")
  }

  fn get_sora_sentinel_token_file_path(&self) -> PathBuf {
    self.path().join("sora_sentinel_token_store.json")
  }
}

impl SoraLegacyCredentialPaths for AppDataRoot {
  fn get_sora_cookie_file_path(&self) -> PathBuf {
    self.credentials_dir().get_sora_cookie_file_path()
  }

  fn get_sora_bearer_token_file_path(&self) -> PathBuf {
    self.credentials_dir().get_sora_bearer_token_file_path()
  }

  fn get_sora_legacy_sentinel_file_path(&self) -> PathBuf {
    self.credentials_dir().get_sora_legacy_sentinel_file_path()
  }

  fn get_sora_sentinel_token_file_path(&self) -> PathBuf {
    self.credentials_dir().get_sora_sentinel_token_file_path()
  }
}

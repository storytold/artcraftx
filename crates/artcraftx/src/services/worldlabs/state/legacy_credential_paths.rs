//! Legacy on-disk credential paths for World Labs.
//!
//! TODO(artcraftx): retire once World Labs moves to the unified TOML
//! credential store in `crate::credentials`.

use crate::state::data_dir::subdirectory::app_credentials_dir::AppCredentialsDir;
use crate::state::data_dir::subdirectory::trait_data_subdir::DataSubdir;
use std::path::PathBuf;

pub trait WorldlabsLegacyCredentialPaths {
  fn get_worldlabs_state_path(&self) -> PathBuf;
  fn get_worldlabs_cookies_path(&self) -> PathBuf;
  fn get_worldlabs_bearer_path(&self) -> PathBuf;
  fn get_worldlabs_refresh_path(&self) -> PathBuf;
}

impl WorldlabsLegacyCredentialPaths for AppCredentialsDir {
  fn get_worldlabs_state_path(&self) -> PathBuf {
    self.path().join("worldlabs_state.json")
  }

  fn get_worldlabs_cookies_path(&self) -> PathBuf {
    self.path().join("worldlabs_cookies.txt")
  }

  fn get_worldlabs_bearer_path(&self) -> PathBuf {
    self.path().join("worldlabs_bearer.txt")
  }

  fn get_worldlabs_refresh_path(&self) -> PathBuf {
    self.path().join("worldlabs_refresh.txt")
  }
}

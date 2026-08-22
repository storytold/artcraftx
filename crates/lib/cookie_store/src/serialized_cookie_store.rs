use crate::cookie_store::CookieStore;
use rfc_cookie_store::Cookie as RfcCookie;
use rfc_cookie_store::CookieStore as RfcCookieStore;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::path::Path;

/// Bumped when the on-disk shape changes. Version 1 was the legacy
/// name/value-only format; version 2 stores full RFC 6265 cookies
/// (domain, path, expiry, flags). Version 1 data does not deserialize —
/// callers already treat parse failures as "no saved cookies".
pub const SERIALIZED_COOKIE_STORE_VERSION: u32 = 2;

/// The serializable snapshot of a [`CookieStore`]. Embed this in state
/// structs or write it to its own file with [`Self::write_to_file`].
#[derive(Serialize, Deserialize)]
pub struct SerializableCookieStore {
  #[serde(default)]
  pub version: u32,
  cookies: Vec<RfcCookie<'static>>,
}

#[derive(Debug)]
pub enum SerializableCookieStoreError {
  IoError(std::io::Error),
  SerializationError(serde_json::Error),
}

impl Error for SerializableCookieStoreError {}

impl Display for SerializableCookieStoreError {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::IoError(err) => write!(f, "IO error: {}", err),
      Self::SerializationError(err) => write!(f, "Serialization error: {}", err),
    }
  }
}

impl SerializableCookieStore {
  pub fn from_cookie_store(store: &CookieStore) -> Self {
    let cookies = store.rfc_store()
        .iter_unexpired()
        .cloned()
        .collect();
    Self {
      version: SERIALIZED_COOKIE_STORE_VERSION,
      cookies,
    }
  }

  pub fn read_from_file<P: AsRef<Path>>(
    file_path: P,
  ) -> Result<Self, SerializableCookieStoreError> {
    let content = std::fs::read_to_string(file_path)
        .map_err(SerializableCookieStoreError::IoError)?;
    serde_json::from_str(&content)
        .map_err(SerializableCookieStoreError::SerializationError)
  }

  pub fn write_to_file<P: AsRef<Path>>(
    &self,
    file_path: P,
  ) -> Result<(), SerializableCookieStoreError> {
    let serialized = serde_json::to_string(self)
        .map_err(SerializableCookieStoreError::SerializationError)?;
    std::fs::write(file_path, serialized)
        .map_err(SerializableCookieStoreError::IoError)
  }

  pub fn to_cookie_store(&self) -> CookieStore {
    let cookies = self.cookies
        .iter()
        .cloned()
        .map(Ok::<RfcCookie<'static>, Infallible>);
    let store = match RfcCookieStore::from_cookies(cookies, false) {
      Ok(store) => store,
      Err(infallible) => match infallible {},
    };
    CookieStore::from_rfc_store(store)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use url::Url;

  #[test]
  fn round_trips_through_json() {
    let site_url = Url::parse("https://www.example.com/").unwrap();
    let mut store = CookieStore::empty();
    store.insert_named("session", "abc123", &site_url);
    store.apply_set_cookie_header("pref=dark; Domain=example.com; Path=/", &site_url);

    let serialized = serde_json::to_string(&store.to_serializable()).unwrap();
    let deserialized: SerializableCookieStore = serde_json::from_str(&serialized).unwrap();
    let restored = deserialized.to_cookie_store();

    assert_eq!(restored.len(), 2);
    assert_eq!(restored.get_cookie_value("session"), Some("abc123"));
    assert_eq!(restored.get_cookie_value("pref"), Some("dark"));
  }

  #[test]
  fn legacy_version_1_data_fails_to_parse() {
    let legacy_json = r#"{"cookies":[{"name":"session","value":"abc"}]}"#;
    let result = serde_json::from_str::<SerializableCookieStore>(legacy_json);
    assert!(result.is_err());
  }
}

//! Serde support for [`CookieStore`] and [`SharedCookieStore`].
//!
//! Both types implement `Serialize` and `Deserialize` directly, so callers
//! can embed a cookie store in their own serializable structs (JSON state
//! files, TOML credential files, etc.) without a conversion type:
//!
//! ```ignore
//! #[derive(Serialize, Deserialize)]
//! struct MyProviderState {
//!   #[serde(skip_serializing_if = "Option::is_none")]
//!   user_cookies: Option<CookieStore>,
//! }
//! ```
//!
//! The wire shape is a versioned envelope: `{ version, cookies: [...] }`,
//! where each cookie carries its full RFC 6265 attributes (domain, path,
//! expiry, flags). Expired cookies are dropped on both save and load. The
//! change log is debug state and is not serialized.

use crate::cookie_store::CookieStore;
use crate::shared_cookie_store::SharedCookieStore;
use ::cookie_store::Cookie as CookieRfc;
use ::cookie_store::CookieStore as CookieStoreRfc;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::convert::Infallible;

/// Bumped when the wire shape changes. Version 1 was the legacy
/// name/value-only format; version 2 stores full RFC 6265 cookies.
/// Version 1 data does not deserialize — callers already treat parse
/// failures as "no saved cookies".
pub const COOKIE_STORE_SERDE_VERSION: u32 = 2;

/// The on-the-wire shape of a serialized [`CookieStore`].
#[derive(Serialize, Deserialize)]
struct CookieStoreEnvelope {
  #[serde(default)]
  version: u32,
  cookies: Vec<CookieRfc<'static>>,
}

impl Serialize for CookieStore {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    let envelope = CookieStoreEnvelope {
      version: COOKIE_STORE_SERDE_VERSION,
      cookies: self.rfc_store().iter_unexpired().cloned().collect(),
    };
    envelope.serialize(serializer)
  }
}

impl<'de> Deserialize<'de> for CookieStore {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let envelope = CookieStoreEnvelope::deserialize(deserializer)?;
    let cookies = envelope.cookies
        .into_iter()
        .map(Ok::<CookieRfc<'static>, Infallible>);
    let store = match CookieStoreRfc::from_cookies(cookies, false) {
      Ok(store) => store,
      Err(infallible) => match infallible {},
    };
    Ok(CookieStore::from_rfc_store(store))
  }
}

/// Serializes as a point-in-time snapshot of the shared store.
impl Serialize for SharedCookieStore {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    self.with_read(|store| store.serialize(serializer))
  }
}

/// Deserializes into a fresh handle, unrelated to any existing clones.
impl<'de> Deserialize<'de> for SharedCookieStore {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    Ok(SharedCookieStore::new(CookieStore::deserialize(deserializer)?))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_derive::{Deserialize, Serialize};
  use url::Url;

  #[test]
  fn cookie_store_round_trips_through_json() {
    let store = example_store();

    let serialized = serde_json::to_string(&store).unwrap();
    let restored: CookieStore = serde_json::from_str(&serialized).unwrap();

    assert_eq!(restored.len(), 2);
    assert_eq!(restored.get_cookie_value("session"), Some("abc123"));
    assert_eq!(restored.get_cookie_value("pref"), Some("dark"));
  }

  #[test]
  fn cookie_store_nests_inside_caller_toml_structs() {
    #[derive(Serialize, Deserialize)]
    struct CredentialFile {
      service: String,
      cookies: CookieStore,
    }

    let file = CredentialFile {
      service: "midjourney_cookies".to_string(),
      cookies: example_store(),
    };

    let serialized = toml::to_string(&file).unwrap();
    let restored: CredentialFile = toml::from_str(&serialized).unwrap();

    assert_eq!(restored.service, "midjourney_cookies");
    assert_eq!(restored.cookies.get_cookie_value("session"), Some("abc123"));
    assert_eq!(restored.cookies.get_cookie_value("pref"), Some("dark"));
  }

  #[test]
  fn shared_cookie_store_round_trips_through_json() {
    let shared = SharedCookieStore::new(example_store());

    let serialized = serde_json::to_string(&shared).unwrap();
    let restored: SharedCookieStore = serde_json::from_str(&serialized).unwrap();

    assert!(restored.has_cookie("session"));
  }

  #[test]
  fn legacy_version_1_data_fails_to_parse() {
    let legacy_json = r#"{"cookies":[{"name":"session","value":"abc"}]}"#;
    let result = serde_json::from_str::<CookieStore>(legacy_json);
    assert!(result.is_err());
  }

  fn example_store() -> CookieStore {
    let site_url = Url::parse("https://www.example.com/").unwrap();
    let mut store = CookieStore::empty();
    store.insert_named("session", "abc123", &site_url);
    store.apply_set_cookie_header("pref=dark; Domain=example.com; Path=/", &site_url);
    store
  }
}

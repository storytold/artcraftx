use chrono::{DateTime, Utc};
use cookie_store_wrapper::cookie_store::CookieStore;
use grok_consumer_statsig::statsig_cache_file::CapturedStatsig;
use serde_derive::{Deserialize, Serialize};

/// A browser-cookie secret for a website integration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CookieCredential {
  /// Last time the cookies were rewritten (e.g. refreshed by the app).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub updated_at: Option<DateTime<Utc>>,

  /// Last time a request with these cookies failed (e.g. session expired).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub failed_at: Option<DateTime<Utc>>,

  /// Last time a request with these cookies succeeded.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub succeeded_at: Option<DateTime<Utc>>,

  /// When the stored [`statsig`](Self::statsig) pieces were captured. Grok only.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub statsig_fetched_at: Option<DateTime<Utc>>,

  /// When the statsig pieces should be re-captured (30 min after fetch). Grok only.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub statsig_refresh_at: Option<DateTime<Utc>>,

  /// Preemptively captured `x-statsig-id` pieces (Grok only). Decoded from a
  /// real browser request during login via `grok_consumer_statsig`.
  /// NB: a table, so it must stay after all scalar fields (before `cookies`).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub statsig: Option<CapturedStatsig>,

  /// The stored cookies with their RFC 6265 attributes.
  /// NB: Serialized as an array of tables, so this field must stay LAST for
  /// the TOML credential files to remain valid.
  pub cookies: CookieStore,
}

impl CookieCredential {
  pub fn new(cookies: CookieStore) -> Self {
    Self {
      updated_at: None,
      failed_at: None,
      succeeded_at: None,
      statsig_fetched_at: None,
      statsig_refresh_at: None,
      statsig: None,
      cookies,
    }
  }

  /// The stored cookies rendered as a `Cookie:` request-header string.
  pub fn cookie_header(&self) -> String {
    self.cookies.to_cookie_string()
  }
}

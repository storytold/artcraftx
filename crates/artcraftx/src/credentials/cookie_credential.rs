use crate::credentials::cookie_credential_grok_extra_pieces::CookieCredentialGrokExtraPieces;
use chrono::{DateTime, Utc};
use cookie_store_wrapper::cookie_store::CookieStore;
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

  /// The User-Agent of the browser that captured these cookies. Bot-protection
  /// cookies (Cloudflare `cf_clearance`, DataDome `datadome`) are bound to
  /// it, so HTTP clients replaying the cookies must present the same string.
  /// Absent for hand-written files and captures from before this was
  /// recorded; clients then fall back to their pinned per-site UA.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub user_agent: Option<String>,

  /// Grok-only extras: the reusable statsig material and its refresh schedule.
  /// NB: a table, so it must stay after all scalar fields (before `cookies`).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub grok_data: Option<CookieCredentialGrokExtraPieces>,

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
      user_agent: None,
      grok_data: None,
      cookies,
    }
  }

  /// The stored cookies rendered as a `Cookie:` request-header string.
  pub fn cookie_header(&self) -> String {
    self.cookies.to_cookie_string()
  }
}

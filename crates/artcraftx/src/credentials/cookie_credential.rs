use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};

/// A browser-cookie secret for a website integration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CookieCredential {
  /// Entire cookie header payload (all cookies), as sent in the
  /// `Cookie:` HTTP request header.
  pub cookie_header: String,

  /// Last time the cookies were rewritten (e.g. refreshed by the app).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub updated_at: Option<DateTime<Utc>>,

  /// Last time a request with these cookies failed (e.g. session expired).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub failed_at: Option<DateTime<Utc>>,

  /// Last time a request with these cookies succeeded.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub succeeded_at: Option<DateTime<Utc>>,
}

impl CookieCredential {
  pub fn new(cookie_header: impl Into<String>) -> Self {
    Self {
      cookie_header: cookie_header.into(),
      updated_at: None,
      failed_at: None,
      succeeded_at: None,
    }
  }
}

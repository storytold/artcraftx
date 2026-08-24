use chrono::{DateTime, Utc};
use grok_consumer_statsig::StatsigMaterial;
use serde_derive::{Deserialize, Serialize};

/// Grok-specific extras stored alongside the cookies in a
/// [`CookieCredential`](crate::credentials::cookie_credential::CookieCredential).
///
/// Holds the reusable statsig material (the seed a fresh `x-statsig-id` is
/// minted from for any endpoint — see [`StatsigMaterial`]) plus when it was
/// captured and when it should be re-captured.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CookieCredentialGrokExtraPieces {
  /// When the statsig material was captured.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub statsig_fetched_at: Option<DateTime<Utc>>,

  /// When the statsig material should be re-captured (30 min after fetch).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub statsig_refresh_at: Option<DateTime<Utc>>,

  /// The reusable statsig material (currently just the captured seed).
  /// NB: a table, so it must stay after the scalar timestamp fields.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub statsig_material: Option<StatsigMaterial>,
}

impl CookieCredentialGrokExtraPieces {
  /// Stamp fresh material with a fetch time and a refresh time
  /// `refresh_minutes` into the future.
  pub fn fresh(material: StatsigMaterial, now: DateTime<Utc>, refresh_minutes: i64) -> Self {
    Self {
      statsig_fetched_at: Some(now),
      statsig_refresh_at: Some(now + chrono::Duration::minutes(refresh_minutes)),
      statsig_material: Some(material),
    }
  }
}

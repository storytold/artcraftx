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

  /// Whether the statsig material is missing or its refresh time has passed, so
  /// it should be re-captured.
  pub fn is_stale(&self, now: DateTime<Utc>) -> bool {
    match (&self.statsig_material, self.statsig_refresh_at) {
      (Some(_), Some(refresh_at)) => now >= refresh_at,
      _ => true, // no material, or no refresh time recorded
    }
  }
}

/// Whether a cookie credential's Grok statsig material is absent or stale and
/// should be re-captured. `None` (no `grok_data` at all) counts as stale.
pub fn grok_statsig_needs_refresh(
  grok_data: Option<&CookieCredentialGrokExtraPieces>,
  now: DateTime<Utc>,
) -> bool {
  grok_data.map_or(true, |data| data.is_stale(now))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn material() -> StatsigMaterial {
    StatsigMaterial::from_seed_b64("7LKU1SbbPgGmlkhdNLb4ltpqrlenNVC2KnvBRD5ly/VV9ueQboie/eZABqiclvh1")
  }

  #[test]
  fn missing_grok_data_is_stale() {
    assert!(grok_statsig_needs_refresh(None, Utc::now()));
  }

  #[test]
  fn fresh_material_is_not_stale_until_refresh_time() {
    let now = Utc::now();
    let data = CookieCredentialGrokExtraPieces::fresh(material(), now, 30);
    assert!(!data.is_stale(now));
    assert!(!data.is_stale(now + chrono::Duration::minutes(29)));
    assert!(data.is_stale(now + chrono::Duration::minutes(30)));
  }

  #[test]
  fn material_without_refresh_time_is_stale() {
    let data = CookieCredentialGrokExtraPieces {
      statsig_fetched_at: None,
      statsig_refresh_at: None,
      statsig_material: Some(material()),
    };
    assert!(data.is_stale(Utc::now()));
  }
}

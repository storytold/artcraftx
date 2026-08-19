use core_types::enums::generation_source::GenerationSource;
use serde_derive::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// A website we can drive a cookie-based web login for.
///
/// The frontend passes one of these to the `open_web_login_command`; the
/// login window module then knows which URLs to visit and which
/// [`GenerationSource`] to save the resulting cookies under.
///
/// The string values are serialized to/from the frontend. NEVER change an
/// existing value; only add new ones.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum LoginWebsite {
  #[serde(rename = "artcraft")]
  ArtCraft,
  #[serde(rename = "openart")]
  OpenArt,
  #[serde(rename = "higgsfield")]
  Higgsfield,
  #[serde(rename = "runway")]
  Runway,
  #[serde(rename = "magnific")]
  Magnific,
  #[serde(rename = "midjourney")]
  Midjourney,
  #[serde(rename = "xai")]
  XAi,
}

impl LoginWebsite {
  /// The credential service (and cookie file `service = "..."`) that a
  /// successful login for this website is stored under.
  pub fn credential_service(&self) -> GenerationSource {
    match self {
      Self::ArtCraft => GenerationSource::ArtcraftCookies,
      Self::OpenArt => GenerationSource::OpenArtCookies,
      Self::Higgsfield => GenerationSource::HiggsfieldCookies,
      Self::Runway => GenerationSource::RunwayCookies,
      Self::Magnific => GenerationSource::MagnificCookies,
      Self::Midjourney => GenerationSource::MidjourneyCookies,
      Self::XAi => GenerationSource::XAiCookies,
    }
  }

  /// The generation provider impacted by this login, if any. Used to tell the
  /// frontend which account state to refresh. `None` means "not tied to a
  /// known generation provider" (refresh broadly).
  pub fn generation_provider(&self) -> Option<GenerationSource> {
    match self {
      Self::ArtCraft => Some(GenerationSource::Artcraft),
      Self::Midjourney => Some(GenerationSource::Midjourney),
      Self::XAi => Some(GenerationSource::Grok),
      Self::OpenArt | Self::Higgsfield | Self::Runway | Self::Magnific => None,
    }
  }

  pub fn to_str(&self) -> &'static str {
    match self {
      Self::ArtCraft => "artcraft",
      Self::OpenArt => "openart",
      Self::Higgsfield => "higgsfield",
      Self::Runway => "runway",
      Self::Magnific => "magnific",
      Self::Midjourney => "midjourney",
      Self::XAi => "xai",
    }
  }
}

impl Display for LoginWebsite {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.to_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn serializes_to_stable_string() {
    let json = serde_json::to_string(&LoginWebsite::ArtCraft).unwrap();
    assert_eq!(json, "\"artcraft\"");
    let back: LoginWebsite = serde_json::from_str("\"higgsfield\"").unwrap();
    assert_eq!(back, LoginWebsite::Higgsfield);
  }

  #[test]
  fn maps_to_cookie_service_type() {
    assert_eq!(
      LoginWebsite::ArtCraft.credential_service(),
      GenerationSource::ArtcraftCookies,
    );
    assert_eq!(
      LoginWebsite::OpenArt.credential_service(),
      GenerationSource::OpenArtCookies,
    );
    assert_eq!(
      LoginWebsite::Higgsfield.credential_service(),
      GenerationSource::HiggsfieldCookies,
    );
    assert_eq!(
      LoginWebsite::Midjourney.credential_service(),
      GenerationSource::MidjourneyCookies,
    );
  }
}

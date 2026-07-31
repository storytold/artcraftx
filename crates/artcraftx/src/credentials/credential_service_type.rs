use serde_derive::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Whether a service authenticates with browser cookies or an API key.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum CredentialKind {
  #[serde(rename = "cookies")]
  Cookies,
  #[serde(rename = "api_key")]
  ApiKey,
}

/// The service (and auth mechanism) a stored credential belongs to.
///
/// The string values are stored in user-editable TOML files.
/// NEVER change existing values; only add new ones.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum CredentialServiceType {
  // Website (cookie) integrations
  #[serde(rename = "artcraft_cookies")]
  ArtcraftCookies,
  #[serde(rename = "grok_cookies")]
  GrokCookies,
  #[serde(rename = "higgsfield_cookies")]
  HiggsfieldCookies,
  #[serde(rename = "magnific_cookies")]
  MagnificCookies,
  #[serde(rename = "midjourney_cookies")]
  MidjourneyCookies,
  #[serde(rename = "openart_cookies")]
  OpenArtCookies,
  #[serde(rename = "runway_cookies")]
  RunwayCookies,
  #[serde(rename = "sora_cookies")]
  SoraCookies,
  #[serde(rename = "worldlabs_cookies")]
  WorldLabsCookies,
  #[serde(rename = "xai_cookies")]
  XAiCookies,

  // API key integrations
  #[serde(rename = "artcraft_api")]
  ArtcraftApi,
  #[serde(rename = "fal_api")]
  FalApi,
  #[serde(rename = "openai_api")]
  OpenAiApi,
  #[serde(rename = "replicate_api")]
  ReplicateApi,
  #[serde(rename = "xai_api")]
  XAiApi,
}

impl CredentialServiceType {
  /// The auth mechanism this service uses.
  pub fn kind(&self) -> CredentialKind {
    match self {
      Self::ArtcraftCookies
      | Self::GrokCookies
      | Self::HiggsfieldCookies
      | Self::MagnificCookies
      | Self::MidjourneyCookies
      | Self::OpenArtCookies
      | Self::RunwayCookies
      | Self::SoraCookies
      | Self::WorldLabsCookies
      | Self::XAiCookies => CredentialKind::Cookies,
      Self::ArtcraftApi
      | Self::FalApi
      | Self::OpenAiApi
      | Self::ReplicateApi
      | Self::XAiApi => CredentialKind::ApiKey,
    }
  }

  pub fn to_str(&self) -> &'static str {
    match self {
      Self::ArtcraftCookies => "artcraft_cookies",
      Self::GrokCookies => "grok_cookies",
      Self::HiggsfieldCookies => "higgsfield_cookies",
      Self::MagnificCookies => "magnific_cookies",
      Self::MidjourneyCookies => "midjourney_cookies",
      Self::OpenArtCookies => "openart_cookies",
      Self::RunwayCookies => "runway_cookies",
      Self::SoraCookies => "sora_cookies",
      Self::WorldLabsCookies => "worldlabs_cookies",
      Self::XAiCookies => "xai_cookies",
      Self::ArtcraftApi => "artcraft_api",
      Self::FalApi => "fal_api",
      Self::OpenAiApi => "openai_api",
      Self::ReplicateApi => "replicate_api",
      Self::XAiApi => "xai_api",
    }
  }
}

impl Display for CredentialServiceType {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.to_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn serializes_to_snake_case_string() {
    let json = serde_json::to_string(&CredentialServiceType::HiggsfieldCookies).unwrap();
    assert_eq!(json, "\"higgsfield_cookies\"");
  }

  #[test]
  fn round_trips_through_serde() {
    let all = [
      CredentialServiceType::ArtcraftCookies,
      CredentialServiceType::GrokCookies,
      CredentialServiceType::HiggsfieldCookies,
      CredentialServiceType::MagnificCookies,
      CredentialServiceType::MidjourneyCookies,
      CredentialServiceType::OpenArtCookies,
      CredentialServiceType::RunwayCookies,
      CredentialServiceType::SoraCookies,
      CredentialServiceType::WorldLabsCookies,
      CredentialServiceType::XAiCookies,
      CredentialServiceType::ArtcraftApi,
      CredentialServiceType::FalApi,
      CredentialServiceType::OpenAiApi,
      CredentialServiceType::ReplicateApi,
      CredentialServiceType::XAiApi,
    ];
    for service in all {
      let json = serde_json::to_string(&service).unwrap();
      let back: CredentialServiceType = serde_json::from_str(&json).unwrap();
      assert_eq!(back, service);
      assert_eq!(json, format!("\"{}\"", service.to_str()));
    }
  }

  #[test]
  fn kind_matches_variant_family() {
    assert_eq!(CredentialServiceType::RunwayCookies.kind(), CredentialKind::Cookies);
    assert_eq!(CredentialServiceType::FalApi.kind(), CredentialKind::ApiKey);
  }
}

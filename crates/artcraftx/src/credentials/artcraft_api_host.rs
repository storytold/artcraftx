use artcraft_client::utils::api_host::ApiHost;

use crate::credentials::credential_service_type::CredentialServiceType;

/// Port the local ArtCraft development server listens on.
pub const ARTCRAFT_LOCAL_DEV_PORT: u32 = 12345;

/// The API host an ArtCraft-family credential authenticates against.
/// `None` for services that aren't ArtCraft accounts.
pub fn maybe_artcraft_api_host_for_service(
  service: CredentialServiceType,
) -> Option<ApiHost> {
  match service {
    CredentialServiceType::Artcraft
    | CredentialServiceType::ArtcraftCookies => Some(ApiHost::Storyteller),
    CredentialServiceType::ArtcraftLocal => {
      Some(ApiHost::Localhost { port: ARTCRAFT_LOCAL_DEV_PORT })
    }
    _ => None,
  }
}

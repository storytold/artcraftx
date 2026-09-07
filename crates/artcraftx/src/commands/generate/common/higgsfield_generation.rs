//! Shared pieces of the first-party (cookie-session) Higgsfield enqueue path
//! for images and videos: the router client for a stored credential, the
//! media-token → CDN URL map for cloud-library references (local files and
//! bytes go straight to Higgsfield without touching ArtCraft), and running a
//! built request through the draft phase.

use std::collections::HashMap;

use artcraft_client::utils::api_host::ApiHost;
use log::{error, info, warn};
use router::api::asset_upload_cache::AssetUploadCache;
use router::client::router_client::RouterClient;
use router::client::router_higgsfield_client::RouterHiggsfieldClient;
use router::errors::artcraft_router_error::ArtcraftRouterError;
use router::errors::provider_error::ProviderError;
use router::generate::generate_image::generate_image_request_builder::GenerateImageRequestBuilder;
use router::generate::generate_image::generate_image_response::GenerateImageResponse;
use router::generate::generate_image::image_generation_draft_context::ImageGenerationDraftContext;
use router::generate::generate_image::image_generation_draft_or_request::ImageGenerationDraftOrRequest;
use router::generate::generate_video::generate_video_request_builder::GenerateVideoRequestBuilder;
use router::generate::generate_video::generate_video_response::GenerateVideoResponse;
use router::generate::generate_video::video_generation_draft_context::VideoGenerationDraftContext;
use router::generate::generate_video::video_generation_draft_or_request::VideoGenerationDraftOrRequest;
use sqlite_identifiers::ids::media_file_token::MediaFileToken;

use crate::commands::generate::generate_error::{CredentialProblemReason, GenerateError};
use crate::commands::generate::generate_image::utils::map_media_files_to_urls::map_media_file_tokens_to_cdn_urls;
use crate::credentials::auth_credential::AuthCredential;
use crate::services::higgsfield::higgsfield_session_from_credential::higgsfield_session_from_credential;

/// Batch job ids are stored on the task as one comma-separated
/// `provider_job_id`, so the poller can follow every job of the set.
pub const HIGGSFIELD_JOB_ID_SEPARATOR: char = ',';

/// The router client for a stored Higgsfield credential. A credential whose
/// session Higgsfield already rejected fails here, before any request, until
/// the user logs in again.
pub fn higgsfield_router_client(credential: &AuthCredential) -> Result<RouterClient, GenerateError> {
  if credential.needs_relogin() {
    info!("Higgsfield credential {} is marked as needing a re-login; not calling the API", credential.id);
    return Err(session_expired(credential));
  }
  let session = higgsfield_session_from_credential(credential)?;
  Ok(RouterClient::Higgsfield(RouterHiggsfieldClient::new(session)))
}

/// The error for a credential whose session is dead.
pub fn session_expired(credential: &AuthCredential) -> GenerateError {
  GenerateError::CredentialProblem(CredentialProblemReason::SessionExpired {
    credential_id: credential.id.to_string(),
    service: credential.service,
  })
}

/// Convert a router error from a Higgsfield call. When Higgsfield says the
/// session is dead (`needs_browser_reauth()`), the credential file is marked
/// so nothing else retries it, and the error becomes a session-expired
/// credential problem (which the frontend turns into a "log in again" prompt).
fn higgsfield_error_to_generate_error(credential: &AuthCredential, err: ArtcraftRouterError) -> GenerateError {
  let session_is_dead = matches!(
    &err,
    ArtcraftRouterError::Provider(ProviderError::Higgsfield(higgsfield_err)) if higgsfield_err.needs_browser_reauth()
  );
  if !session_is_dead {
    return GenerateError::from(err);
  }
  warn!("Higgsfield rejected the session of credential {}; marking it as needing a re-login: {:?}", credential.id, err);
  let mut marked = credential.clone();
  if let Err(save_err) = marked.mark_relogin_required() {
    error!("Could not mark credential {} as needing a re-login: {}", credential.id, save_err);
  }
  session_expired(credential)
}

/// Resolve reference media tokens (cloud-library picks only) to their
/// ArtCraft CDN URLs. Higgsfield can't fetch ArtCraft media itself, so the
/// router downloads each URL and re-uploads the bytes as Higgsfield
/// reference media. Empty when every reference is a local file or bytes —
/// then ArtCraft is never contacted at all.
pub async fn higgsfield_media_url_map(tokens: &[MediaFileToken]) -> Result<HashMap<MediaFileToken, String>, GenerateError> {
  if tokens.is_empty() {
    return Ok(HashMap::new());
  }
  let urls = map_media_file_tokens_to_cdn_urls(tokens, &ApiHost::Storyteller).await?;
  Ok(tokens.iter().cloned().zip(urls).collect())
}

/// Build, finalize (uploading references) and send an image request.
pub async fn send_higgsfield_image_request(
  credential: &AuthCredential,
  builder: GenerateImageRequestBuilder,
  client: &RouterClient,
  media_url_map: &HashMap<MediaFileToken, String>,
  upload_cache: &dyn AssetUploadCache,
) -> Result<GenerateImageResponse, GenerateError> {
  let request = match builder.build2().map_err(|err| {
    warn!("Could not build Higgsfield image request: {:?}", err);
    GenerateError::from(err)
  })? {
    ImageGenerationDraftOrRequest::Request(request) => request,
    ImageGenerationDraftOrRequest::Draft(draft) => {
      info!("Higgsfield image request has references; uploading them first");
      let context = ImageGenerationDraftContext {
        client: Some(client),
        media_file_to_artcraft_url_map: Some(media_url_map),
        asset_upload_cache: Some(upload_cache),
      };
      draft.finalize(context).await.map_err(|err| {
        warn!("Could not upload references to Higgsfield: {:?}", err);
        higgsfield_error_to_generate_error(credential, err)
      })?
    }
  };

  request.send_request(client).await.map_err(|err| {
    warn!("Higgsfield image generation failed: {:?}", err);
    higgsfield_error_to_generate_error(credential, err)
  })
}

/// Build, finalize (uploading keyframes and references) and send a video
/// request.
pub async fn send_higgsfield_video_request(
  credential: &AuthCredential,
  builder: GenerateVideoRequestBuilder,
  client: &RouterClient,
  media_url_map: &HashMap<MediaFileToken, String>,
  upload_cache: &dyn AssetUploadCache,
) -> Result<GenerateVideoResponse, GenerateError> {
  let request = match builder.build2().map_err(|err| {
    warn!("Could not build Higgsfield video request: {:?}", err);
    GenerateError::from(err)
  })? {
    VideoGenerationDraftOrRequest::Request(request) => request,
    VideoGenerationDraftOrRequest::Draft(draft) => {
      info!("Higgsfield video request has media; uploading it first");
      let context = VideoGenerationDraftContext {
        client: Some(client),
        media_file_to_artcraft_url_map: Some(media_url_map),
        character_token_to_kinovi_id_map: None,
        asset_upload_cache: Some(upload_cache),
      };
      draft.finalize(context).await.map_err(|err| {
        warn!("Could not upload media to Higgsfield: {:?}", err);
        higgsfield_error_to_generate_error(credential, err)
      })?
    }
  };

  request.send_request(client).await.map_err(|err| {
    warn!("Higgsfield video generation failed: {:?}", err);
    higgsfield_error_to_generate_error(credential, err)
  })
}

/// One `provider_job_id` for a Higgsfield job set.
pub fn join_higgsfield_job_ids(job_ids: &[String]) -> String {
  job_ids.join(&HIGGSFIELD_JOB_ID_SEPARATOR.to_string())
}

/// The job ids stored on a task by [`join_higgsfield_job_ids`].
pub fn split_higgsfield_job_ids(provider_job_id: &str) -> Vec<String> {
  provider_job_id
      .split(HIGGSFIELD_JOB_ID_SEPARATOR)
      .map(str::trim)
      .filter(|id| !id.is_empty())
      .map(str::to_string)
      .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn job_ids_round_trip() {
    let ids = vec!["job_a".to_string(), "job_b".to_string()];
    let joined = join_higgsfield_job_ids(&ids);
    assert_eq!(joined, "job_a,job_b");
    assert_eq!(split_higgsfield_job_ids(&joined), ids);
    assert_eq!(split_higgsfield_job_ids("solo"), vec!["solo".to_string()]);
    assert!(split_higgsfield_job_ids(" , ").is_empty());
  }
}

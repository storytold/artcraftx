//! Shared pieces of the first-party (cookie-session) Higgsfield enqueue path
//! for images and videos: the router client for a stored credential, the
//! media-token → CDN URL map the router needs to re-upload references to
//! Higgsfield, and running a built request through the draft phase.

use std::collections::HashMap;

use artcraft_client::utils::api_host::ApiHost;
use log::{info, warn};
use router::client::router_client::RouterClient;
use router::client::router_higgsfield_client::RouterHiggsfieldClient;
use router::generate::generate_image::generate_image_request_builder::GenerateImageRequestBuilder;
use router::generate::generate_image::generate_image_response::GenerateImageResponse;
use router::generate::generate_image::image_generation_draft_context::ImageGenerationDraftContext;
use router::generate::generate_image::image_generation_draft_or_request::ImageGenerationDraftOrRequest;
use router::generate::generate_video::generate_video_request_builder::GenerateVideoRequestBuilder;
use router::generate::generate_video::generate_video_response::GenerateVideoResponse;
use router::generate::generate_video::video_generation_draft_context::VideoGenerationDraftContext;
use router::generate::generate_video::video_generation_draft_or_request::VideoGenerationDraftOrRequest;
use sqlite_identifiers::ids::media_file_token::MediaFileToken;

use crate::commands::generate::generate_error::GenerateError;
use crate::commands::generate::generate_image::utils::map_media_files_to_urls::map_media_file_tokens_to_cdn_urls;
use crate::credentials::auth_credential::AuthCredential;
use crate::services::higgsfield::higgsfield_session_from_credential::higgsfield_session_from_credential;

/// Batch job ids are stored on the task as one comma-separated
/// `provider_job_id`, so the poller can follow every job of the set.
pub const HIGGSFIELD_JOB_ID_SEPARATOR: char = ',';

/// The router client for a stored Higgsfield credential.
pub fn higgsfield_router_client(credential: &AuthCredential) -> Result<RouterClient, GenerateError> {
  let session = higgsfield_session_from_credential(credential)?;
  Ok(RouterClient::Higgsfield(RouterHiggsfieldClient::new(session)))
}

/// Resolve reference media tokens to their ArtCraft CDN URLs. Higgsfield
/// can't fetch ArtCraft media itself, so the router downloads each URL and
/// re-uploads the bytes as Higgsfield reference media.
pub async fn higgsfield_media_url_map(tokens: &[MediaFileToken]) -> Result<HashMap<MediaFileToken, String>, GenerateError> {
  if tokens.is_empty() {
    return Ok(HashMap::new());
  }
  let urls = map_media_file_tokens_to_cdn_urls(tokens, &ApiHost::Storyteller).await?;
  Ok(tokens.iter().cloned().zip(urls).collect())
}

/// Build, finalize (uploading references) and send an image request.
pub async fn send_higgsfield_image_request(
  builder: GenerateImageRequestBuilder,
  client: &RouterClient,
  media_url_map: &HashMap<MediaFileToken, String>,
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
      };
      draft.finalize(context).await.map_err(|err| {
        warn!("Could not upload references to Higgsfield: {:?}", err);
        GenerateError::from(err)
      })?
    }
  };

  request.send_request(client).await.map_err(|err| {
    warn!("Higgsfield image generation failed: {:?}", err);
    GenerateError::from(err)
  })
}

/// Build, finalize (uploading keyframes and references) and send a video
/// request.
pub async fn send_higgsfield_video_request(
  builder: GenerateVideoRequestBuilder,
  client: &RouterClient,
  media_url_map: &HashMap<MediaFileToken, String>,
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
      };
      draft.finalize(context).await.map_err(|err| {
        warn!("Could not upload media to Higgsfield: {:?}", err);
        GenerateError::from(err)
      })?
    }
  };

  request.send_request(client).await.map_err(|err| {
    warn!("Higgsfield video generation failed: {:?}", err);
    GenerateError::from(err)
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

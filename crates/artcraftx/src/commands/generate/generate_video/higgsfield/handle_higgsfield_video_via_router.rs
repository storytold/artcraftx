//! Video generation on the user's own Higgsfield account, via the router's
//! first-party Higgsfield provider.

use core_types::enums::generation_source::GenerationSource;
use log::info;
use router::api::audio_list_ref::AudioListRef;
use router::api::image_list_ref::ImageListRef;
use router::api::image_ref::ImageRef;
use router::api::router_provider::RouterProvider;
use router::api::video_list_ref::VideoListRef;
use router::client::request_mismatch_mitigation_strategy::RequestMismatchMitigationStrategy;
use router::generate::generate_video::generate_video_request_builder::GenerateVideoRequestBuilder;
use sqlite_identifiers::enums::task_type::TaskType;
use sqlite_identifiers::ids::media_file_token::MediaFileToken;

use crate::commands::generate::common::higgsfield_generation::{
  higgsfield_media_url_map, higgsfield_router_client, join_higgsfield_job_ids, send_higgsfield_video_request,
};
use crate::commands::generate::generate_error::GenerateError;
use crate::commands::generate::generate_video::request::TauriGenerateVideoRequest;
use crate::commands::generate::task_enqueue_success::TaskEnqueueSuccess;
use crate::commands::utils::api_adapters::models::video::tauri_video_model_to_generation_model::tauri_video_model_to_generation_model;
use crate::commands::utils::api_adapters::models::video::tauri_video_model_to_router_model::tauri_video_model_to_router_model;
use crate::credentials::auth_credential::AuthCredential;

/// Enqueue via the router's first-party Higgsfield provider.
///
/// Keyframes and image / video / audio references travel as ArtCraft media
/// tokens; the router resolves them to CDN URLs through the map built here,
/// downloads them, and uploads them to Higgsfield (running its IP check
/// where the model demands it) before enqueuing. The returned task carries
/// every job id of the Higgsfield job set so the poller can follow a batch.
pub async fn handle_higgsfield_video_via_router(
  request: &TauriGenerateVideoRequest,
  credential: &AuthCredential,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let tauri_model = request.model.ok_or(GenerateError::no_model_specified())?;
  let router_model = tauri_video_model_to_router_model(tauri_model);
  let generation_model = tauri_video_model_to_generation_model(tauri_model);

  let media_url_map = higgsfield_media_url_map(&collect_media_tokens(request)).await?;

  // Character references have no Higgsfield equivalent; the router drops
  // them with a warning, so they aren't forwarded at all.
  let router_request = GenerateVideoRequestBuilder {
    model: router_model,
    provider: RouterProvider::Higgsfield,
    prompt: request.prompt.clone(),
    negative_prompt: request.negative_prompt.clone(),
    start_frame: request.start_frame_image_media_token.clone().map(ImageRef::MediaFileToken),
    end_frame: request.end_frame_image_media_token.clone().map(ImageRef::MediaFileToken),
    reference_images: request.reference_image_media_tokens.clone().map(ImageListRef::MediaFileTokens),
    reference_videos: request.reference_video_media_tokens.clone().map(VideoListRef::MediaFileTokens),
    reference_audio: request.reference_audio_media_tokens.clone().map(AudioListRef::MediaFileTokens),
    reference_character_tokens: None,
    resolution: request.resolution,
    aspect_ratio: request.aspect_ratio,
    bitrate: None,
    duration_seconds: request.duration_seconds,
    video_batch_count: request.video_batch_count,
    generate_audio: request.generate_audio,
    request_mismatch_mitigation_strategy: RequestMismatchMitigationStrategy::PayMoreUpgrade,
    idempotency_token: None,
  };

  let client = higgsfield_router_client(credential)?;
  let response = send_higgsfield_video_request(router_request, &client, &media_url_map).await?;

  let payload = response
      .get_higgsfield_payload()
      .ok_or(GenerateError::ResponseHadNoJobTokens)?;

  info!(
    "Higgsfield video generation enqueued: job_set={} jobs={:?}",
    payload.job_set_id, payload.job_ids,
  );

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::VideoGeneration,
    model: Some(generation_model),
    // NB: the task uses the bare `Higgsfield` provider (what the polling
    // thread queries), not the `HiggsfieldCookies` credential service.
    provider: GenerationSource::Higgsfield,
    provider_job_id: Some(join_higgsfield_job_ids(&payload.job_ids)),
    is_batch_generation: payload.job_ids.len() > 1,
    maybe_queue_status_url: None,
    maybe_queue_response_url: None,
    maybe_prompt_token: None,
  })
}

/// Every media token the request references, so each can be resolved to a
/// CDN URL for re-upload.
fn collect_media_tokens(request: &TauriGenerateVideoRequest) -> Vec<MediaFileToken> {
  let mut tokens: Vec<MediaFileToken> = Vec::new();
  tokens.extend(request.start_frame_image_media_token.clone());
  tokens.extend(request.end_frame_image_media_token.clone());
  tokens.extend(request.reference_image_media_tokens.clone().unwrap_or_default());
  tokens.extend(request.reference_video_media_tokens.clone().unwrap_or_default());
  tokens.extend(request.reference_audio_media_tokens.clone().unwrap_or_default());
  tokens.dedup();
  tokens
}

/// Live smoke tests that hit the REAL Higgsfield API with the REAL stored
/// credential and SPEND HIGGSFIELD CREDITS. `#[ignore]`; run explicitly:
///   SQLX_OFFLINE=true cargo test -p artcraftx live_higgsfield_video -- --ignored --nocapture
#[cfg(test)]
mod live_higgsfield_video_tests {
  use router::api::router_aspect_ratio::RouterAspectRatio;
  use router::api::router_resolution::RouterResolution;

  use crate::commands::generate::generate_video::request::TauriVideoModel;
  use crate::state::data_dir::app_data_root::AppDataRoot;

  use super::*;

  /// A production ArtCraft media file (see `test_data::web::image_media_tokens`).
  const JUNO_AT_LAKE_MEDIA_TOKEN: &str = "m_m1bz02z1kkzanxy6rb4vk1kvq9de9g";

  fn higgsfield_credential() -> AuthCredential {
    let app_data_root = AppDataRoot::create_default().expect("app data root");
    app_data_root
        .credentials_dir()
        .load_credentials()
        .expect("load credentials")
        .into_iter()
        .find(|c| c.service == GenerationSource::HiggsfieldCookies)
        .expect("no `higgsfield_cookies` credential on disk; log into Higgsfield via the app first")
  }

  #[tokio::test]
  #[ignore] // live: spends Higgsfield credits
  async fn live_higgsfield_video_text_to_video() {
    let credential = higgsfield_credential();
    let request = TauriGenerateVideoRequest {
      credential_id: Some(credential.id.as_str().to_string()),
      model: Some(TauriVideoModel::Seedance2p0Mini),
      prompt: Some("a shiba inu surfing a big wave, action photo".to_string()),
      aspect_ratio: Some(RouterAspectRatio::WideSixteenByNine),
      resolution: Some(RouterResolution::FourEightyP),
      duration_seconds: Some(4),
      video_batch_count: Some(1),
      ..Default::default()
    };

    let success = handle_higgsfield_video_via_router(&request, &credential).await.expect("enqueue should succeed");
    println!("[live] Higgsfield video enqueued: provider_job_id={:?}", success.provider_job_id);
    assert_eq!(success.provider, GenerationSource::Higgsfield);
    assert!(success.provider_job_id.is_some());
  }

  #[tokio::test]
  #[ignore] // live: spends Higgsfield credits and uploads a start frame (with IP check)
  async fn live_higgsfield_video_from_start_frame() {
    let credential = higgsfield_credential();
    let request = TauriGenerateVideoRequest {
      credential_id: Some(credential.id.as_str().to_string()),
      model: Some(TauriVideoModel::Seedance2p0Mini),
      prompt: Some("gentle camera push-in, ripples on the water".to_string()),
      start_frame_image_media_token: Some(MediaFileToken::new_from_str(JUNO_AT_LAKE_MEDIA_TOKEN)),
      resolution: Some(RouterResolution::FourEightyP),
      duration_seconds: Some(4),
      ..Default::default()
    };

    let success = handle_higgsfield_video_via_router(&request, &credential).await.expect("enqueue should succeed");
    println!("[live] Higgsfield image-to-video enqueued: provider_job_id={:?}", success.provider_job_id);
    assert!(success.provider_job_id.is_some());
  }
}

use sqlite_identifiers::enums::generation_provider::GenerationProvider;
use sqlite_identifiers::enums::task_type::TaskType;
use log::{info, warn};
use router::api::image_list_ref::ImageListRef;
use router::api::router_provider::RouterProvider;
use router::client::generation_mode_mismatch_strategy::GenerationModeMismatchStrategy;
use router::client::request_mismatch_mitigation_strategy::RequestMismatchMitigationStrategy;
use router::client::router_artcraft_client::RouterArtcraftClient;
use router::client::router_client::RouterClient;
use router::generate::generate_image::generate_image_request_builder::GenerateImageRequestBuilder;
use router::generate::generate_image::image_generation_draft_or_request::ImageGenerationDraftOrRequest;
use sqlite_identifiers::ids::media_file_token::MediaFileToken;

use crate::commands::utils::api_adapters::models::image::tauri_image_model_to_generation_model::tauri_image_model_to_generation_model;
use crate::commands::utils::api_adapters::models::image::tauri_image_model_to_router_model::tauri_image_model_to_router_model;
use crate::commands::generate::generate_error::GenerateError;
use crate::commands::generate::task_enqueue_success::TaskEnqueueSuccess;
use crate::commands::generate::common::generation_credential::{credential_not_usable, resolve_generation_credential, storyteller_creds_from_credential};
use crate::commands::generate::generate_image::handle_fal_credential::handle_fal_credential;
use crate::commands::generate::generate_image::utils::convert_enums_to_router::{convert_aspect_ratio, convert_quality, convert_resolution};
use crate::commands::generate::generate_image::tauri_generate_image_request::TauriGenerateImageRequest;
use crate::commands::generate::generate_image::utils::parse_semantic_media_files::{parse_semantic_media_files, SemanticMediaFiles};
use crate::credentials::artcraft_api_host::maybe_artcraft_api_host_for_service;
use crate::credentials::credential::Credential;
use crate::credentials::credential_service_type::CredentialServiceType;
use crate::state::data_dir::app_data_root::AppDataRoot;

/// Credential-driven image generation: resolve the stored credential named
/// by the request's `credential_id`, then route to that credential's service
/// via the router. The router owns upload + dispatch mechanics per provider.
pub async fn handle_credential_router(
  request: &TauriGenerateImageRequest,
  app_data_root: &AppDataRoot,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let credential = resolve_generation_credential(
    request.credential_id.as_deref(),
    app_data_root,
  )?;

  info!(
    "handle_credential_router: credential={} service={}",
    credential.id.as_str(),
    credential.service,
  );

  match credential.service {
    CredentialServiceType::Artcraft
    | CredentialServiceType::ArtcraftLocal
    | CredentialServiceType::ArtcraftCookies => {
      handle_artcraft_credential(request, &credential).await
    }
    CredentialServiceType::FalApi => {
      let api_key = credential.api_key().ok_or_else(|| {
        credential_not_usable(&credential, "the FAL credential has no API key")
      })?;
      handle_fal_credential(
        request,
        &api_key.api_key,
      ).await
    }
    other => Err(credential_not_usable(
      &credential,
      &format!("accounts for service {} can't generate images yet", other),
    )),
  }
}

/// Generate via the router's Artcraft provider, authenticating with the
/// stored session cookies. Raw canvas/scene/mask bytes are uploaded to
/// ArtCraft first (with the same credential) so the router request only
/// carries media tokens.
async fn handle_artcraft_credential(
  request: &TauriGenerateImageRequest,
  credential: &Credential,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let tauri_model = request.model.ok_or(GenerateError::no_model_specified())?;

  let router_model = tauri_image_model_to_router_model(tauri_model)
    .ok_or(GenerateError::NotYetImplemented(
      format!("Model {:?} can't be generated with this account yet", tauri_model),
    ))?;

  let api_host = maybe_artcraft_api_host_for_service(credential.service)
      .expect("caller only routes ArtCraft services here");

  let creds = storyteller_creds_from_credential(credential)?;

  // Upload any raw image bytes before the generate request.
  let semantic_media_files = parse_semantic_media_files(
    request,
    &creds,
    &api_host,
  ).await?;

  let image_inputs = collect_image_inputs(request, &semantic_media_files);

  let router_request = GenerateImageRequestBuilder {
    model: router_model,
    provider: RouterProvider::Artcraft,
    prompt: request.prompt.clone(),
    image_inputs,
    resolution: request.resolution.map(convert_resolution),
    aspect_ratio: request.aspect_ratio.map(convert_aspect_ratio),
    quality: request.quality.map(convert_quality),
    image_batch_count: request.batch_size.map(|n| n as u16),
    horizontal_angle: request.adjust_horizontal_angle,
    vertical_angle: request.adjust_vertical_angle,
    zoom: request.adjust_zoom,
    request_mismatch_mitigation_strategy: RequestMismatchMitigationStrategy::PayMoreUpgrade,
    generation_mode_mismatch_strategy: Some(GenerationModeMismatchStrategy::GenerateAnyway),
    idempotency_token: None,
  };

  let client = RouterClient::Artcraft(RouterArtcraftClient::new(
    api_host,
    creds,
  ));

  info!("Building Artcraft image generation plan: model={:?}", router_model);

  let generation_request = match router_request.build2() {
    Ok(ImageGenerationDraftOrRequest::Request(generation_request)) => generation_request,
    Ok(ImageGenerationDraftOrRequest::Draft(draft)) => {
      warn!("Artcraft build unexpectedly produced a draft: {:?}", draft);
      return Err(GenerateError::NotYetImplemented(
        "Artcraft requests should not require a draft phase".to_string(),
      ));
    }
    Err(err) => {
      warn!("Could not build Artcraft router request: {:?}", err);
      return Err(err.into());
    }
  };

  let response = generation_request.send_request(&client).await
      .map_err(|err| {
        warn!("Artcraft router image generation failed: {:?}", err);
        GenerateError::from(err)
      })?;

  let payload = response.get_artcraft_payload()
      .ok_or(GenerateError::ResponseHadNoJobTokens)?;

  info!(
    "Router image generation enqueued: inference_job_token={}, response={:?}",
    payload.inference_job_token.as_str(),
    response,
  );

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::ImageGeneration,
    model: Some(tauri_image_model_to_generation_model(tauri_model)),
    provider: GenerationProvider::Artcraft,
    provider_job_id: Some(payload.inference_job_token.to_string()),
    maybe_queue_status_url: None,
    maybe_queue_response_url: None,
    maybe_prompt_token: None,
  })
}

/// Reference images for the router request: canvas first, then scene, then
/// the un-semantic reference images. (Artcraft accepts media tokens
/// directly; no URL resolution needed.)
fn collect_image_inputs(
  request: &TauriGenerateImageRequest,
  semantic_media_files: &SemanticMediaFiles,
) -> Option<ImageListRef> {
  let mut tokens: Vec<MediaFileToken> = Vec::new();

  if let Some(canvas_token) = &semantic_media_files.canvas_image_media_token {
    tokens.push(canvas_token.clone());
  }

  if let Some(scene_token) = &semantic_media_files.scene_image_media_token {
    tokens.push(scene_token.clone());
  }

  if let Some(media_tokens) = &request.image_media_tokens {
    tokens.extend(media_tokens.clone());
  }

  if tokens.is_empty() {
    None
  } else {
    Some(ImageListRef::MediaFileTokens(tokens))
  }
}


/// Live smoke tests that hit the REAL production API with the REAL stored
/// credential and SPEND CREDITS. All `#[ignore]`; run explicitly:
///   SQLX_OFFLINE=true cargo test -p artcraftx live_generation -- --ignored --nocapture
#[cfg(test)]
mod live_generation_tests {
  use artcraft_client::utils::api_host::ApiHost;

  use crate::commands::generate::generate_image::tauri_image_model::TauriImageModel;

  use super::*;

  fn production_setup() -> (AppDataRoot, String) {
    let app_data_root = AppDataRoot::create_default().expect("app data root");
    let credential_id = app_data_root
        .credentials_dir()
        .load_credentials()
        .expect("load credentials")
        .into_iter()
        .find(|c| c.service == CredentialServiceType::Artcraft)
        .expect("no `artcraft` (production) credential on disk")
        .id
        .as_str()
        .to_string();
    (app_data_root, credential_id)
  }

  #[tokio::test]
  #[ignore] // live: spends credits on api.storyteller.ai
  async fn live_generation_image_nano_banana() {
    let (app_data_root, credential_id) = production_setup();

    let request = TauriGenerateImageRequest {
      credential_id: Some(credential_id),
      model: Some(TauriImageModel::NanoBanana),
      prompt: Some("a tiny friendly robot watering a bonsai tree".to_string()),
      batch_size: Some(1),
      ..Default::default()
    };

    let result = handle_credential_router(&request, &app_data_root).await;
    let success = result.expect("nano banana enqueue should succeed");
    println!("[live] nano banana enqueued: job_id={:?}", success.provider_job_id);
    assert!(success.provider_job_id.is_some());
  }

  #[tokio::test]
  #[ignore] // live: spends credits on api.storyteller.ai
  async fn live_generation_image_midjourney_8() {
    let (app_data_root, credential_id) = production_setup();

    let request = TauriGenerateImageRequest {
      credential_id: Some(credential_id),
      model: Some(TauriImageModel::Midjourney8),
      prompt: Some("a lighthouse on a cliff at dusk, oil painting".to_string()),
      ..Default::default()
    };

    let result = handle_credential_router(&request, &app_data_root).await;
    match result {
      Ok(success) => {
        println!("[live] midjourney 8 enqueued: job_id={:?}", success.provider_job_id);
        assert!(success.provider_job_id.is_some());
      }
      Err(err) => panic!("midjourney 8 enqueue failed: {:?}", err),
    }
  }
}

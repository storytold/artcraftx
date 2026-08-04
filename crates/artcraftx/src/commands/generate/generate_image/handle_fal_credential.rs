use artcraft_client::utils::api_host::ApiHost;
use sqlite_identifiers::generation_provider::GenerationProvider;
use sqlite_identifiers::task_type::TaskType;
use log::{info, warn};
use router::api::image_list_ref::ImageListRef;
use router::api::router_provider::RouterProvider;
use router::client::generation_mode_mismatch_strategy::GenerationModeMismatchStrategy;
use router::client::request_mismatch_mitigation_strategy::RequestMismatchMitigationStrategy;
use router::client::router_client::RouterClient;
use router::client::router_fal_client::RouterFalClient;
use router::generate::generate_image::generate_image_request_builder::GenerateImageRequestBuilder;
use router::generate::generate_image::generate_image_response::GenerateImageResponse;
use router::generate::generate_image::image_generation_draft_or_request::ImageGenerationDraftOrRequest;
use sqlite_identifiers::media_file_token::MediaFileToken;

use crate::commands::utils::api_adapters::models::image::tauri_image_model_to_generation_model::tauri_image_model_to_generation_model;
use crate::commands::utils::api_adapters::models::image::tauri_image_model_to_router_model::tauri_image_model_to_router_model;
use crate::commands::generate::generate_error::GenerateError;
use crate::commands::generate::task_enqueue_success::TaskEnqueueSuccess;
use crate::commands::generate::generate_image::tauri_generate_image_request::TauriGenerateImageRequest;
use crate::commands::generate::generate_image::tauri_image_model::TauriImageModel;
use crate::commands::generate::generate_image::utils::convert_enums_to_router::{convert_aspect_ratio, convert_quality, convert_resolution};
use crate::commands::generate::generate_image::utils::map_media_files_to_urls::map_media_file_tokens_to_cdn_urls;

/// Generate via the router's FAL provider using a stored FAL API key
/// credential. FAL only accepts URLs, so Artcraft media tokens are resolved
/// to CDN URLs first.
pub async fn handle_fal_credential(
  request: &TauriGenerateImageRequest,
  api_key: &str,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let tauri_model = request.model.ok_or(GenerateError::no_model_specified())?;

  let router_model = tauri_image_model_to_router_model(tauri_model)
    .ok_or(GenerateError::NotYetImplemented(
      format!("Model {:?} is not supported via the FAL router path", tauri_model),
    ))?;

  let image_inputs = resolve_image_inputs(request, &ApiHost::Storyteller).await?;

  let router_request = GenerateImageRequestBuilder {
    model: router_model,
    provider: RouterProvider::Fal,
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

  let fal_client = RouterFalClient::new_polling_only_from_raw_key(api_key);
  let client = RouterClient::Fal(fal_client);

  info!("Building FAL image generation plan: model={:?}", router_model);

  let generation_request = match router_request.build2() {
    Ok(ImageGenerationDraftOrRequest::Request(generation_request)) => generation_request,
    Ok(ImageGenerationDraftOrRequest::Draft(draft)) => {
      warn!("FAL build unexpectedly produced a draft: {:?}", draft);
      return Err(GenerateError::NotYetImplemented(
        "FAL requests should not require a draft phase".to_string(),
      ));
    }
    Err(err) => {
      warn!("Could not build FAL router request: {:?}", err);
      return Err(err.into());
    }
  };

  info!("Executing FAL image generation. Request: {:?}", generation_request);

  let response = generation_request.send_request(&client).await
      .map_err(|err| {
        warn!("FAL image generation failed: {:?}", err);
        GenerateError::from(err)
      })?;

  build_task_enqueue_success(tauri_model, response)
}

// ── Helpers ──

async fn resolve_image_inputs(
  request: &TauriGenerateImageRequest,
  api_host: &ApiHost,
) -> Result<Option<ImageListRef>, GenerateError> {
  let mut tokens: Vec<MediaFileToken> = Vec::new();

  if let Some(canvas_token) = &request.canvas_image_media_token {
    tokens.push(canvas_token.clone());
  }

  if let Some(scene_token) = &request.scene_image_media_token {
    tokens.push(scene_token.clone());
  }

  if let Some(media_tokens) = &request.image_media_tokens {
    tokens.extend(media_tokens.clone());
  }

  if tokens.is_empty() {
    return Ok(None);
  }

  let urls = map_media_file_tokens_to_cdn_urls(&tokens, api_host).await?;
  Ok(Some(ImageListRef::Urls(urls)))
}

fn build_task_enqueue_success(
  tauri_model: TauriImageModel,
  response: GenerateImageResponse,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let fal_payload = response.get_fal_payload()
    .ok_or(GenerateError::ResponseHadNoJobTokens)?;

  let provider_job_id = fal_payload.request_id
    .or(fal_payload.gateway_request_id)
    .ok_or(GenerateError::ResponseHadNoJobTokens)?;

  info!(
    "Router image generation enqueued via FAL: request_id={}, status_url={:?}",
    provider_job_id,
    fal_payload.maybe_status_url,
  );

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::ImageGeneration,
    model: Some(tauri_image_model_to_generation_model(tauri_model)),
    provider: GenerationProvider::Fal,
    provider_job_id: Some(provider_job_id),
    maybe_queue_status_url: fal_payload.maybe_status_url,
    maybe_queue_response_url: fal_payload.maybe_response_url,
    maybe_prompt_token: None,
  })
}

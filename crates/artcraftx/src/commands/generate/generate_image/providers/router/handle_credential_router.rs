use artcraft_client::credentials::storyteller_credential_set::StorytellerCredentialSet;
use artcraft_client::utils::api_host::ApiHost;
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use log::{info, warn};
use router::api::image_list_ref::ImageListRef;
use router::api::router_provider::RouterProvider;
use router::client::generation_mode_mismatch_strategy::GenerationModeMismatchStrategy;
use router::client::request_mismatch_mitigation_strategy::RequestMismatchMitigationStrategy;
use router::client::router_artcraft_client::RouterArtcraftClient;
use router::client::router_client::RouterClient;
use router::generate::generate_image::generate_image_request_builder::GenerateImageRequestBuilder;
use router::generate::generate_image::image_generation_draft_or_request::ImageGenerationDraftOrRequest;
use tokens::tokens::media_files::MediaFileToken;

use crate::api_adapters::models::image::tauri_image_model_to_generation_model::tauri_image_model_to_generation_model;
use crate::api_adapters::models::image::tauri_image_model_to_router_model::tauri_image_model_to_router_model;
use crate::commands::enqueue::generate_error::{CredentialProblemReason, GenerateError};
use crate::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::commands::generate::generate_image::providers::router::handle_api_providers::handle_api_key_provider;
use crate::commands::generate::generate_image::providers::router::utils::convert_enums_to_router::{convert_aspect_ratio, convert_quality, convert_resolution};
use crate::commands::generate::generate_image::tauri_generate_image_request::TauriGenerateImageRequest;
use crate::commands::generate::generate_image::utils::parse_semantic_media_files::{parse_semantic_media_files, SemanticMediaFiles};
use crate::credentials::artcraft_api_host::maybe_artcraft_api_host_for_service;
use crate::credentials::credential::Credential;
use crate::credentials::credential_service_type::CredentialServiceType;
use crate::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;

/// Credential-driven image generation: resolve the stored credential named
/// by the request's `credential_id`, then route to that credential's service
/// via the router. The router owns upload + dispatch mechanics per provider.
pub async fn handle_credential_router(
  request: &TauriGenerateImageRequest,
  app_data_root: &AppDataRoot,
  app_env_configs: &AppEnvConfigs,
  storyteller_creds_manager: &StorytellerCredentialManager,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let credential = resolve_credential(request, app_data_root)?;

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
      handle_api_key_provider(
        request,
        GenerationProvider::Fal,
        &api_key.api_key,
        app_env_configs,
        storyteller_creds_manager,
      ).await
    }
    other => Err(credential_not_usable(
      &credential,
      &format!("accounts for service {} can't generate images yet", other),
    )),
  }
}

/// Require and load the credential named by the request.
fn resolve_credential(
  request: &TauriGenerateImageRequest,
  app_data_root: &AppDataRoot,
) -> Result<Credential, GenerateError> {
  let credential_id = request.credential_id.as_deref()
      .filter(|id| !id.trim().is_empty())
      .ok_or(GenerateError::CredentialProblem(
        CredentialProblemReason::NoCredentialSupplied,
      ))?;

  let maybe_credential = app_data_root
      .credentials_dir()
      .find_credential_by_id(credential_id)
      .map_err(GenerateError::from)?;

  maybe_credential.ok_or_else(|| {
    GenerateError::CredentialProblem(CredentialProblemReason::CredentialNotFound {
      credential_id: credential_id.to_string(),
    })
  })
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

/// Rebuild the web-session credential set from the stored cookie header.
fn storyteller_creds_from_credential(
  credential: &Credential,
) -> Result<StorytellerCredentialSet, GenerateError> {
  let cookie = credential.cookies().ok_or_else(|| {
    credential_not_usable(credential, "the account has no session cookies")
  })?;

  StorytellerCredentialSet::parse_multi_cookie_header(&cookie.cookie_header)
      .map_err(|err| {
        credential_not_usable(
          credential,
          &format!("the stored session cookies could not be parsed ({})", err),
        )
      })?
      .filter(|creds| !creds.is_empty())
      .ok_or_else(|| {
        credential_not_usable(credential, "the stored session cookies are empty")
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

fn credential_not_usable(credential: &Credential, reason: &str) -> GenerateError {
  GenerateError::CredentialProblem(CredentialProblemReason::CredentialNotUsable {
    credential_id: credential.id.to_string(),
    reason: reason.to_string(),
  })
}

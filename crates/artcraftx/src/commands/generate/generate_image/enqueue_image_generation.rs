use artcraft_client::utils::api_host::ApiHost;
use core_types::enums::generation_source::GenerationSource;
use log::{info, warn};
use midjourney_client::recipes::get_user_info::{get_user_info, GetUserInfoArgs};
use router::api::image_list_ref::ImageListRef;
use router::api::router_provider::RouterProvider;
use router::client::generation_mode_mismatch_strategy::GenerationModeMismatchStrategy;
use router::client::request_mismatch_mitigation_strategy::RequestMismatchMitigationStrategy;
use router::client::router_artcraft_client::RouterArtcraftClient;
use router::client::router_client::RouterClient;
use router::client::router_fal_client::RouterFalClient;
use router::client::router_midjourney_client::RouterMidjourneyClient;
use router::generate::generate_image::generate_image_request_builder::GenerateImageRequestBuilder;
use router::generate::generate_image::generate_image_response::GenerateImageResponse;
use router::generate::generate_image::image_generation_draft_or_request::ImageGenerationDraftOrRequest;
use router::generate::generate_image::image_generation_request::ImageGenerationRequest;
use sqlite_identifiers::enums::task_type::TaskType;
use sqlite_identifiers::ids::media_file_token::MediaFileToken;

use crate::commands::generate::common::generation_credential::{
  credential_not_usable, resolve_generation_credential, storyteller_creds_from_credential,
};
use crate::commands::generate::generate_error::GenerateError;
use crate::commands::generate::generate_image::tauri_generate_image_request::TauriGenerateImageRequest;
use crate::commands::generate::generate_image::tauri_image_model::TauriImageModel;
use crate::commands::generate::generate_image::utils::convert_enums_to_router::{
  convert_aspect_ratio, convert_quality, convert_resolution,
};
use crate::commands::generate::generate_image::utils::map_media_files_to_urls::map_media_file_tokens_to_cdn_urls;
use crate::commands::generate::generate_image::utils::parse_semantic_media_files::{
  parse_semantic_media_files, SemanticMediaFiles,
};
use crate::commands::generate::task_enqueue_success::TaskEnqueueSuccess;
use crate::commands::utils::api_adapters::models::image::tauri_image_model_to_generation_model::tauri_image_model_to_generation_model;
use crate::commands::utils::api_adapters::models::image::tauri_image_model_to_router_model::tauri_image_model_to_router_model;
use crate::credentials::auth_credential::AuthCredential;
use crate::services::midjourney::state::midjourney_live_session::MidjourneyLiveSession;
use crate::services::midjourney::utils::extract_midjourney_user_id_from_cookies::extract_midjourney_user_id_from_cookie_header;
use crate::services::midjourney::utils::midjourney_browser_profile::midjourney_browser_profile;
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::utils::services::artcraft_api_host::maybe_artcraft_api_host_for_service;

/// Credential-driven image generation: resolve the stored credential named by
/// the request's `credential_id`, then invoke the router for that credential's
/// service. This is the single enqueue entry point — there is no separate
/// per-provider "handler" layer.
pub async fn enqueue_image_generation(
  request: &TauriGenerateImageRequest,
  app_data_root: &AppDataRoot,
  mj_session: &MidjourneyLiveSession,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let credential = resolve_generation_credential(request.credential_id.as_deref(), app_data_root)?;

  info!(
    "enqueue_image_generation: credential={} service={}",
    credential.id.as_str(),
    credential.service,
  );

  match credential.service {
    GenerationSource::Artcraft
    | GenerationSource::ArtcraftLocal
    | GenerationSource::ArtcraftCookies => enqueue_via_artcraft(request, &credential).await,

    GenerationSource::FalApi => {
      let api_key = credential.api_key().ok_or_else(|| {
        credential_not_usable(&credential, "the FAL credential has no API key")
      })?;
      enqueue_via_fal(request, &api_key.api_key).await
    }

    GenerationSource::MidjourneyCookies | GenerationSource::Midjourney => {
      enqueue_via_midjourney(request, &credential, mj_session).await
    }

    other => Err(credential_not_usable(
      &credential,
      &format!("accounts for service {} can't generate images yet", other),
    )),
  }
}

// ── Artcraft ──

/// Generate via the router's Artcraft provider, authenticating with the stored
/// session cookies. Raw canvas/scene/mask bytes are uploaded to ArtCraft first
/// (with the same credential) so the router request only carries media tokens.
async fn enqueue_via_artcraft(
  request: &TauriGenerateImageRequest,
  credential: &AuthCredential,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let tauri_model = request.model.ok_or(GenerateError::no_model_specified())?;

  let router_model = tauri_image_model_to_router_model(tauri_model).ok_or(
    GenerateError::NotYetImplemented(format!(
      "Model {:?} can't be generated with this account yet",
      tauri_model
    )),
  )?;

  let api_host = maybe_artcraft_api_host_for_service(credential.service)
      .expect("caller only routes ArtCraft services here");

  let creds = storyteller_creds_from_credential(credential)?;

  // Upload any raw image bytes before the generate request.
  let semantic_media_files = parse_semantic_media_files(request, &creds, &api_host).await?;
  let image_inputs = collect_artcraft_image_inputs(request, &semantic_media_files);

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

  let client = RouterClient::Artcraft(RouterArtcraftClient::new(api_host, creds));
  let generation_request = build_direct_request(router_request, "Artcraft")?;

  let response = generation_request.send_request(&client).await.map_err(|err| {
    warn!("Artcraft router image generation failed: {:?}", err);
    GenerateError::from(err)
  })?;

  let payload = response
      .get_artcraft_payload()
      .ok_or(GenerateError::ResponseHadNoJobTokens)?;

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::ImageGeneration,
    model: Some(tauri_image_model_to_generation_model(tauri_model)),
    provider: GenerationSource::Artcraft,
    provider_job_id: Some(payload.inference_job_token.to_string()),
    maybe_queue_status_url: None,
    maybe_queue_response_url: None,
    maybe_prompt_token: None,
  })
}

/// Reference images for the Artcraft router request: canvas first, then scene,
/// then the un-semantic reference images. (Artcraft accepts media tokens
/// directly; no URL resolution needed.)
fn collect_artcraft_image_inputs(
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

// ── Fal ──

/// Generate via the router's FAL provider using a stored FAL API key. FAL only
/// accepts URLs, so Artcraft media tokens are resolved to CDN URLs first.
async fn enqueue_via_fal(
  request: &TauriGenerateImageRequest,
  api_key: &str,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let tauri_model = request.model.ok_or(GenerateError::no_model_specified())?;

  let router_model = tauri_image_model_to_router_model(tauri_model).ok_or(
    GenerateError::NotYetImplemented(format!(
      "Model {:?} is not supported via the FAL router path",
      tauri_model
    )),
  )?;

  let image_inputs = resolve_fal_image_inputs(request, &ApiHost::Storyteller).await?;

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

  let client = RouterClient::Fal(RouterFalClient::new_polling_only_from_raw_key(api_key));
  let generation_request = build_direct_request(router_request, "FAL")?;

  let response = generation_request.send_request(&client).await.map_err(|err| {
    warn!("FAL image generation failed: {:?}", err);
    GenerateError::from(err)
  })?;

  build_fal_task_enqueue_success(tauri_model, response)
}

async fn resolve_fal_image_inputs(
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

fn build_fal_task_enqueue_success(
  tauri_model: TauriImageModel,
  response: GenerateImageResponse,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let fal_payload = response
      .get_fal_payload()
      .ok_or(GenerateError::ResponseHadNoJobTokens)?;

  let provider_job_id = fal_payload
      .request_id
      .or(fal_payload.gateway_request_id)
      .ok_or(GenerateError::ResponseHadNoJobTokens)?;

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::ImageGeneration,
    model: Some(tauri_image_model_to_generation_model(tauri_model)),
    provider: GenerationSource::Fal,
    provider_job_id: Some(provider_job_id),
    maybe_queue_status_url: fal_payload.maybe_status_url,
    maybe_queue_response_url: fal_payload.maybe_response_url,
    maybe_prompt_token: None,
  })
}

// ── Midjourney (first-party, cookie-session) ──

/// Generate via the router's first-party Midjourney provider, authenticating
/// with the stored session cookies. The `user_id` (needed to form the
/// submit channel) is taken from the in-memory session, resolving it once via
/// the index page when absent.
async fn enqueue_via_midjourney(
  request: &TauriGenerateImageRequest,
  credential: &AuthCredential,
  mj_session: &MidjourneyLiveSession,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let tauri_model = request.model.ok_or(GenerateError::no_model_specified())?;

  let router_model = tauri_image_model_to_router_model(tauri_model).ok_or(
    GenerateError::NotYetImplemented(format!(
      "Model {:?} is not supported for a Midjourney account",
      tauri_model
    )),
  )?;

  let cookie = credential.cookies().ok_or_else(|| {
    credential_not_usable(credential, "the Midjourney credential has no cookies")
  })?;
  let cookie_header = cookie.cookie_header();

  let browser = midjourney_browser_profile();

  info!(
    "Midjourney enqueue: cookie_header_len={}, has_auth_i={}, has_auth_r={}, has_cf_clearance={}, browser={}",
    cookie_header.len(),
    cookie_header.contains("__Host-Midjourney.AuthUserTokenV3_i"),
    cookie_header.contains("__Host-Midjourney.AuthUserTokenV3_r"),
    cookie_header.contains("cf_clearance"),
    browser.label(),
  );

  // Resolve the Midjourney user id (needed for `singleplayer_{user_id}`).
  // Prefer the live session, then the auth cookie's JWT (no network), and only
  // fall back to the Cloudflare-gated index page as a last resort.
  let user_id = match mj_session.user_id() {
    Some(user_id) => user_id,
    None => match extract_midjourney_user_id_from_cookie_header(&cookie_header) {
      Some(user_id) => {
        info!("Resolved Midjourney user id from auth cookie JWT: {}", user_id.as_str());
        mj_session.set_identity(user_id.clone(), None);
        user_id
      }
      None => {
        warn!("Could not read Midjourney user id from the auth cookie; falling back to the index page.");
        let info = get_user_info(GetUserInfoArgs {
          cookie_header: &cookie_header,
          hostname: None,
          browser: Some(browser.clone()),
        })
        .await
        .map_err(|err| {
          warn!("Could not read Midjourney user info from index page: {:?}", err);
          credential_not_usable(
            credential,
            "could not read your Midjourney account info; the session may have expired",
          )
        })?;

        let user_id = info.user_id.ok_or_else(|| {
          credential_not_usable(credential, "Midjourney did not return a user id")
        })?;
        mj_session.set_identity(user_id.clone(), info.websocket_token);
        user_id
      }
    },
  };

  let router_request = GenerateImageRequestBuilder {
    model: router_model,
    provider: RouterProvider::Midjourney,
    prompt: request.prompt.clone(),
    image_inputs: None,
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

  let client = RouterClient::Midjourney(RouterMidjourneyClient::new(
    cookie_header,
    user_id,
    browser,
  ));
  let generation_request = build_direct_request(router_request, "Midjourney")?;

  let response = generation_request.send_request(&client).await.map_err(|err| {
    warn!("Midjourney image generation failed: {:?}", err);
    GenerateError::from(err)
  })?;

  let payload = response
      .get_midjourney_payload()
      .ok_or(GenerateError::ResponseHadNoJobTokens)?;

  info!("Midjourney image generation enqueued: job_id={}", payload.job_id);

  // Stash the prompt so the (metadata-less) websocket completion path can
  // still attribute the created Storyteller prompt.
  if let Some(prompt) = &request.prompt {
    mj_session.record_pending_prompt(payload.job_id.clone(), prompt.clone());
  }

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::ImageGeneration,
    model: Some(tauri_image_model_to_generation_model(tauri_model)),
    // NB: the task uses the bare `Midjourney` provider (what the completion
    // pollers query), not the `MidjourneyCookies` credential service.
    provider: GenerationSource::Midjourney,
    provider_job_id: Some(payload.job_id),
    maybe_queue_status_url: None,
    maybe_queue_response_url: None,
    maybe_prompt_token: None,
  })
}

// ── Shared ──

/// Build a directly-sendable request, rejecting the draft phase (none of these
/// providers use it on the text/token path).
fn build_direct_request(
  router_request: GenerateImageRequestBuilder,
  provider_label: &str,
) -> Result<ImageGenerationRequest, GenerateError> {
  match router_request.build2() {
    Ok(ImageGenerationDraftOrRequest::Request(generation_request)) => Ok(generation_request),
    Ok(ImageGenerationDraftOrRequest::Draft(draft)) => {
      warn!("{} build unexpectedly produced a draft: {:?}", provider_label, draft);
      Err(GenerateError::NotYetImplemented(format!(
        "{} requests should not require a draft phase",
        provider_label
      )))
    }
    Err(err) => {
      warn!("Could not build {} router request: {:?}", provider_label, err);
      Err(err.into())
    }
  }
}

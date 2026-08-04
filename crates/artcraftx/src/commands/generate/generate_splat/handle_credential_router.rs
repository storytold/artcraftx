use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use log::{info, warn};
use router::api::router_provider::RouterProvider;
use router::client::request_mismatch_mitigation_strategy::RequestMismatchMitigationStrategy;
use router::client::router_artcraft_client::RouterArtcraftClient;
use router::client::router_client::RouterClient;
use router::generate::generate_splat::generate_splat_request_builder::GenerateSplatRequestBuilder;
use router::generate::generate_splat::splat_generation_draft_or_request::SplatGenerationDraftOrRequest;
use router::api::image_list_ref::ImageListRef;
use router::api::video_ref::VideoRef;

use crate::api_adapters::models::splat::tauri_splat_model_to_generation_model::tauri_splat_model_to_generation_model;
use crate::api_adapters::models::splat::tauri_splat_model_to_router_model::tauri_splat_model_to_router_model;
use crate::commands::generate::generate_error::GenerateError;
use crate::commands::generate::task_enqueue_success::TaskEnqueueSuccess;
use crate::commands::generate::common::generation_credential::{credential_not_usable, resolve_generation_credential, storyteller_creds_from_credential};
use crate::commands::generate::generate_splat::request::TauriGenerateSplatRequest;
use crate::credentials::artcraft_api_host::maybe_artcraft_api_host_for_service;
use crate::credentials::credential::Credential;
use crate::credentials::credential_service_type::CredentialServiceType;
use crate::state::data_dir::app_data_root::AppDataRoot;

/// Credential-driven splat generation: resolve the stored credential named
/// by the request's `credential_id`, then route to that credential's service
/// via the router. The router owns upload + dispatch mechanics per provider.
pub async fn handle_credential_router(
  request: &TauriGenerateSplatRequest,
  app_data_root: &AppDataRoot,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let credential = resolve_generation_credential(
    request.credential_id.as_deref(),
    app_data_root,
  )?;

  info!(
    "handle_credential_router (splat): credential={} service={}",
    credential.id.as_str(),
    credential.service,
  );

  match credential.service {
    CredentialServiceType::Artcraft
    | CredentialServiceType::ArtcraftLocal
    | CredentialServiceType::ArtcraftCookies => {
      handle_artcraft_credential(request, &credential).await
    }
    other => Err(credential_not_usable(
      &credential,
      &format!("accounts for service {} can't generate splat yet", other),
    )),
  }
}

/// Generate via the router's Artcraft provider, authenticating with the
/// stored session cookies. Requests carry media tokens only, which Artcraft
/// accepts directly — no pre-upload step is needed.
async fn handle_artcraft_credential(
  request: &TauriGenerateSplatRequest,
  credential: &Credential,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let tauri_model = request.model.ok_or(GenerateError::no_model_specified())?;

  let router_model = tauri_splat_model_to_router_model(tauri_model);
  let generation_model = tauri_splat_model_to_generation_model(tauri_model);

  let api_host = maybe_artcraft_api_host_for_service(credential.service)
      .expect("caller only routes ArtCraft services here");

  let creds = storyteller_creds_from_credential(credential)?;

  let client = RouterClient::Artcraft(RouterArtcraftClient::new(
    api_host,
    creds,
  ));

  let router_request = GenerateSplatRequestBuilder {
    model: router_model,
    provider: RouterProvider::Artcraft,
    prompt: request.prompt.clone(),
    reference_images: request.reference_image_media_tokens.clone().map(ImageListRef::MediaFileTokens),
    reference_video: request.reference_video_media_token.clone().map(VideoRef::MediaFileToken),
    is_panoramic: request.is_panoramic,
    disable_recaption: request.disable_recaption,
    request_mismatch_mitigation_strategy: RequestMismatchMitigationStrategy::PayMoreUpgrade,
    idempotency_token: None,
  };

  info!("Building Artcraft splat generation plan: model={:?}", router_model);

  let generation_request = match router_request.build2() {
    Ok(SplatGenerationDraftOrRequest::Request(generation_request)) => generation_request,
    Ok(SplatGenerationDraftOrRequest::Draft(draft)) => {
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
        warn!("Artcraft router splat generation failed: {:?}", err);
        GenerateError::from(err)
      })?;

  let payload = response.get_artcraft_payload()
      .ok_or(GenerateError::ResponseHadNoJobTokens)?;

  info!(
    "Router splat generation enqueued: inference_job_token={}, all_tokens={:?}, response={:?}",
    payload.inference_job_token.as_str(),
    payload.all_inference_job_tokens,
    response,
  );

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::GaussianGeneration,
    model: Some(generation_model),
    provider: GenerationProvider::Artcraft,
    provider_job_id: Some(payload.inference_job_token.to_string()),
    maybe_queue_status_url: None,
    maybe_queue_response_url: None,
    maybe_prompt_token: None,
  })
}

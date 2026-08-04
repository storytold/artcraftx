use log::info;

use crate::api_adapters::models::video::tauri_video_model_to_generation_model::tauri_video_model_to_generation_model;
use crate::api_adapters::models::video::tauri_video_model_to_router_model::tauri_video_model_to_router_model;
use crate::commands::generate::generate_error::GenerateError;
use crate::commands::generate::task_enqueue_success::TaskEnqueueSuccess;
use crate::commands::generate::common::generation_credential::{credential_not_usable, resolve_generation_credential, storyteller_creds_from_credential};
use crate::commands::generate::generate_video::artcraft::handle_artcraft_video_via_router::handle_artcraft_video_via_router;
use crate::commands::generate::generate_video::request::TauriGenerateVideoRequest;
use crate::credentials::artcraft_api_host::maybe_artcraft_api_host_for_service;
use crate::credentials::credential::Credential;
use crate::credentials::credential_service_type::CredentialServiceType;
use crate::state::data_dir::app_data_root::AppDataRoot;

/// Credential-driven video generation: resolve the stored credential named
/// by the request's `credential_id`, then route to that credential's service
/// via the router. The router owns upload + dispatch mechanics per provider.
pub async fn handle_credential_router(
  request: &TauriGenerateVideoRequest,
  app_data_root: &AppDataRoot,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let credential = resolve_generation_credential(
    request.credential_id.as_deref(),
    app_data_root,
  )?;

  info!(
    "handle_credential_router (video): credential={} service={}",
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
      &format!("accounts for service {} can't generate videos yet", other),
    )),
  }
}

/// Generate via the router's Artcraft provider, authenticating with the
/// stored session cookies. Video requests carry media tokens only, which
/// Artcraft accepts directly — no pre-upload step is needed.
async fn handle_artcraft_credential(
  request: &TauriGenerateVideoRequest,
  credential: &Credential,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let tauri_model = request.model.ok_or(GenerateError::no_model_specified())?;

  let router_model = tauri_video_model_to_router_model(tauri_model);
  let generation_model = tauri_video_model_to_generation_model(tauri_model);

  let api_host = maybe_artcraft_api_host_for_service(credential.service)
      .expect("caller only routes ArtCraft services here");

  let creds = storyteller_creds_from_credential(credential)?;

  handle_artcraft_video_via_router(
    request,
    &api_host,
    &creds,
    router_model,
    generation_model,
  ).await
}

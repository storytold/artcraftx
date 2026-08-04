use crate::commands::enqueue::generate_error::GenerateError;
use crate::commands::enqueue::image_to_gaussian::enqueue_image_to_gaussian_command::EnqueueImageToGaussianRequest;
use crate::commands::enqueue::task_enqueue_success::TaskEnqueueSuccess;
use crate::events::generation_events::common::GenerationModel;
use crate::state::app_env_configs::app_env_configs::AppEnvConfigs;
use artcraft_client::credentials::storyteller_credential_set::StorytellerCredentialSet;
use router::api::router_splat_model::RouterSplatModel;
use router::api::image_list_ref::ImageListRef;
use router::api::router_provider::RouterProvider;
use router::client::router_artcraft_client::RouterArtcraftClient;
use router::client::router_client::RouterClient;
use router::generate::generate_splat::generate_splat_request::GenerateSplatRequest;
use enums::common::generation_provider::GenerationProvider;
use enums::tauri::tasks::task_type::TaskType;
use log::{error, info};

pub(super) async fn handle_artcraft_splat_via_router(
  request: &EnqueueImageToGaussianRequest,
  app_env_configs: &AppEnvConfigs,
  creds: &StorytellerCredentialSet,
  model: RouterSplatModel,
  generation_model: GenerationModel,
) -> Result<TaskEnqueueSuccess, GenerateError> {
  let client = RouterClient::Artcraft(RouterArtcraftClient::new(
    app_env_configs.storyteller_host.clone(),
    creds.clone(),
  ));

  let router_request = GenerateSplatRequest {
    model,
    provider: RouterProvider::Artcraft,
    prompt: request.prompt.clone(),
    reference_images: request.image_media_tokens.clone().map(ImageListRef::MediaFileTokens),
    idempotency_token: None,
  };

  let plan = router_request.build()?;

  info!("Splat Generation Plan: {:?}", plan);

  let response = match plan.generate_splat(&client).await {
    Ok(resp) => {
      info!("Successfully enqueued splat generation.");
      resp
    }
    Err(err) => {
      error!("Failed to enqueue splat generation: {:?}", err);
      return Err(GenerateError::from(err));
    }
  };

  let job_id = response.get_artcraft_payload()
      .map(|p| p.inference_job_token.to_string())
      .ok_or(GenerateError::ResponseHadNoJobTokens)?;

  Ok(TaskEnqueueSuccess {
    task_type: TaskType::ImageGeneration,
    model: Some(generation_model),
    provider: GenerationProvider::Artcraft,
    provider_job_id: Some(job_id),
    maybe_queue_status_url: None,
    maybe_prompt_token: None,
    maybe_queue_response_url: None,
  })
}

use crate::commands::response::failure_response_wrapper::{CommandErrorResponseWrapper, CommandErrorStatus};
use crate::commands::response::shorthand::ResponseOrError;
use crate::commands::response::success_response_wrapper::SerializeMarker;
use crate::state::app_env_configs::app_env_configs::AppEnvConfigs;
use artcraft_client::api_defs::omni_gen::cost_and_generate_requests::omni_gen_audio_cost_and_generate_request::OmniGenAudioCostAndGenerateRequest;
use artcraft_client::api_defs::omni_gen::cost_response::omni_gen_audio_cost_response::OmniGenAudioCostResponse;
use artcraft_client::endpoints::omni_gen::cost::audio::omni_gen_audio_cost::{omni_gen_audio_cost, OmniGenAudioCostArgs};
use log::debug;
use serde_derive::Serialize;
use tauri::State;

impl SerializeMarker for OmniGenAudioCostResponse {}

#[derive(Serialize)]
pub struct EstimateAudioCostError {
  pub success: bool,
  pub error_message: String,
}

/// Estimate the cost of a audio generation via the omni cost endpoint.
/// Anonymous: no credentials are needed for a baseline estimate.
#[tauri::command]
pub async fn estimate_audio_cost_command(
  request: OmniGenAudioCostAndGenerateRequest,
  app_env_configs: State<'_, AppEnvConfigs>,
) -> ResponseOrError<OmniGenAudioCostResponse, EstimateAudioCostError> {
  debug!("estimate_audio_cost_command called");

  let result = omni_gen_audio_cost(OmniGenAudioCostArgs {
    api_host: &app_env_configs.storyteller_host,
    api_or_web_creds: None,
    request: &request,
  }).await;

  match result {
    Ok(response) => Ok(response.into()),
    Err(err) => Err(CommandErrorResponseWrapper {
      status: CommandErrorStatus::BadRequest,
      error_message: None,
      error_type: None,
      error_details: Some(EstimateAudioCostError {
        success: false,
        error_message: err.to_string(),
      }),
    }),
  }
}

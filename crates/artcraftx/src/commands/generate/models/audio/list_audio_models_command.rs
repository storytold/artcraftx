//! Tauri command: list every audio model, with capabilities and presentation.
//! Served from the built-in `models` crate table.

use log::info;
use models::configs::audio_model_config::AudioModelConfig;
use models::configs::audio_models::AUDIO_MODELS;
use serde_derive::Serialize;

use crate::commands::utils::response::shorthand::ResponseOrErrorMessage;
use crate::commands::utils::response::success_response_wrapper::SerializeMarker;

#[derive(Clone, Debug, Serialize)]
pub struct ListAudioModelsResponse {
  /// Every model, in picker order. Includes disabled models (`is_disabled`),
  /// which the frontend hides but keeps addressable.
  pub models: Vec<AudioModelConfig>,
}

impl SerializeMarker for ListAudioModelsResponse {}

#[tauri::command]
pub async fn list_audio_models_command() -> ResponseOrErrorMessage<ListAudioModelsResponse> {
  info!("list_audio_models_command called");
  Ok(ListAudioModelsResponse { models: AUDIO_MODELS.clone() }.into())
}

//! Tauri command: list every video model, with capabilities and presentation.
//! Served from the built-in `models` crate table.

use log::info;
use models::configs::video_model_config::VideoModelConfig;
use models::configs::video_models::VIDEO_MODELS;
use serde_derive::Serialize;

use crate::commands::utils::response::shorthand::ResponseOrErrorMessage;
use crate::commands::utils::response::success_response_wrapper::SerializeMarker;

#[derive(Clone, Debug, Serialize)]
pub struct ListVideoModelsResponse {
  /// Every model, in picker order. Includes disabled models (`is_disabled`),
  /// which the frontend hides but keeps addressable.
  pub models: Vec<VideoModelConfig>,
}

impl SerializeMarker for ListVideoModelsResponse {}

#[tauri::command]
pub async fn list_video_models_command() -> ResponseOrErrorMessage<ListVideoModelsResponse> {
  info!("list_video_models_command called");
  Ok(ListVideoModelsResponse { models: VIDEO_MODELS.clone() }.into())
}

//! Tauri command: list every splat model, with capabilities and presentation.
//! Served from the built-in `models` crate table.

use log::info;
use models::configs::splat_model_config::SplatModelConfig;
use models::configs::splat_models::SPLAT_MODELS;
use serde_derive::Serialize;

use crate::commands::utils::response::shorthand::ResponseOrErrorMessage;
use crate::commands::utils::response::success_response_wrapper::SerializeMarker;

#[derive(Clone, Debug, Serialize)]
pub struct ListSplatModelsResponse {
  /// Every model, in picker order. Includes disabled models (`is_disabled`),
  /// which the frontend hides but keeps addressable.
  pub models: Vec<SplatModelConfig>,
}

impl SerializeMarker for ListSplatModelsResponse {}

#[tauri::command]
pub async fn list_splat_models_command() -> ResponseOrErrorMessage<ListSplatModelsResponse> {
  info!("list_splat_models_command called");
  Ok(ListSplatModelsResponse { models: SPLAT_MODELS.clone() }.into())
}

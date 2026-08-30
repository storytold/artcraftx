//! Tauri command: list every mesh model, with capabilities and presentation.
//! Served from the built-in `models` crate table.

use log::info;
use models::configs::mesh_model_config::MeshModelConfig;
use models::configs::mesh_models::MESH_MODELS;
use serde_derive::Serialize;

use crate::commands::utils::response::shorthand::ResponseOrErrorMessage;
use crate::commands::utils::response::success_response_wrapper::SerializeMarker;

#[derive(Clone, Debug, Serialize)]
pub struct ListMeshModelsResponse {
  /// Every model, in picker order. Includes disabled models (`is_disabled`),
  /// which the frontend hides but keeps addressable.
  pub models: Vec<MeshModelConfig>,
}

impl SerializeMarker for ListMeshModelsResponse {}

#[tauri::command]
pub async fn list_mesh_models_command() -> ResponseOrErrorMessage<ListMeshModelsResponse> {
  info!("list_mesh_models_command called");
  Ok(ListMeshModelsResponse { models: MESH_MODELS.clone() }.into())
}

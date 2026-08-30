use crate::state::promptbox::promptbox_modality::PromptboxModality;
use crate::state::promptbox::promptbox_state::PromptboxState;
use crate::state::promptbox::promptbox_state_manager::PromptboxStateManager;
use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use tauri::State;

/// A patch to the prompt box state. Every field is optional and only the
/// present ones change; the frontend sends whatever just changed.
#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct UpdatePromptboxStateRequest {
  /// Which prompt box the account / model / options apply to. Required when
  /// any of those three are present.
  pub modality: Option<PromptboxModality>,
  pub selected_account_id: Option<String>,
  pub selected_model: Option<String>,
  /// Replaces the modality's options wholesale (the frontend owns the shape).
  pub options: Option<Map<String, Value>>,
  /// Merged into `last_account_by_model` (model id -> credential id).
  pub last_account_by_model: Option<BTreeMap<String, String>>,
}

#[derive(Serialize)]
pub struct UpdatePromptboxStateResponse {
  pub state: PromptboxState,
}

#[tauri::command]
pub async fn update_promptbox_state_command(
  request: UpdatePromptboxStateRequest,
  promptbox_state: State<'_, PromptboxStateManager>,
) -> Result<UpdatePromptboxStateResponse, String> {
  info!("update_promptbox_state_command called: {:?}", request);

  let has_modality_fields = request.selected_account_id.is_some()
      || request.selected_model.is_some()
      || request.options.is_some();
  if has_modality_fields && request.modality.is_none() {
    return Err("`modality` is required when updating account, model, or options".to_string());
  }

  let state = promptbox_state
      .update(|state| apply(request, state))
      .map_err(|err| {
        error!("Error updating prompt box state: {:?}", err);
        format!("Error updating prompt box state: {:?}", err)
      })?;

  Ok(UpdatePromptboxStateResponse { state })
}

fn apply(request: UpdatePromptboxStateRequest, state: &mut PromptboxState) {
  if let Some(modality) = request.modality {
    let target = state.modality_mut(modality);
    if let Some(account_id) = request.selected_account_id {
      target.selected_account_id = Some(account_id);
    }
    if let Some(model) = request.selected_model {
      target.selected_model = Some(model);
    }
    if let Some(options) = request.options {
      target.options = options;
    }
  }
  if let Some(entries) = request.last_account_by_model {
    state.last_account_by_model.extend(entries);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn decode(json: &str) -> UpdatePromptboxStateRequest {
    serde_json::from_str(json).unwrap()
  }

  #[test]
  fn patch_only_touches_present_fields() {
    let mut state = PromptboxState::default();
    state.image.selected_account_id = Some("credential_a".to_string());
    state.image.options = json!({ "batch_count": 2 }).as_object().unwrap().clone();

    apply(decode(r#"{ "modality": "image", "selected_model": "midjourney_7" }"#), &mut state);
    assert_eq!(state.image.selected_model.as_deref(), Some("midjourney_7"));
    assert_eq!(state.image.selected_account_id.as_deref(), Some("credential_a"), "untouched");
    assert_eq!(state.image.options["batch_count"], 2, "untouched");

    apply(decode(r#"{ "modality": "image", "options": { "aspect_ratio": "square" } }"#), &mut state);
    assert_eq!(state.image.options.len(), 1, "options replace wholesale");
    assert_eq!(state.image.options["aspect_ratio"], "square");
    assert_eq!(state.video, Default::default(), "other modalities untouched");
  }

  #[test]
  fn last_account_entries_merge() {
    let mut state = PromptboxState::default();
    apply(decode(r#"{ "last_account_by_model": { "midjourney_7": "credential_a" } }"#), &mut state);
    apply(decode(r#"{ "last_account_by_model": { "grok_imagine_image": "credential_g", "midjourney_7": "credential_b" } }"#), &mut state);
    assert_eq!(state.last_account_by_model["midjourney_7"], "credential_b");
    assert_eq!(state.last_account_by_model["grok_imagine_image"], "credential_g");
  }

  #[test]
  fn unknown_modality_is_rejected() {
    assert!(serde_json::from_str::<UpdatePromptboxStateRequest>(r#"{ "modality": "hologram" }"#).is_err());
  }
}

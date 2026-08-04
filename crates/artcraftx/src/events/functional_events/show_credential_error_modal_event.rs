use crate::events::basic_sendable_event_trait::{BasicEventStatus, BasicSendableEvent};
use enums::tauri::ux::tauri_event_name::TauriEventName;
use serde_derive::Serialize;

/// Tell the frontend to show a dismissable modal explaining a credential
/// problem: the request named no credential, the credential id didn't match
/// anything on disk, or the credential can't be used for the request.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ShowCredentialErrorModalEvent {
  pub message: String,
}

impl BasicSendableEvent for ShowCredentialErrorModalEvent {
  const FRONTEND_EVENT_NAME: TauriEventName = TauriEventName::ShowCredentialErrorModalEvent;
  const EVENT_STATUS: BasicEventStatus = BasicEventStatus::Failure;
}

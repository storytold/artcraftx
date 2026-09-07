use crate::events::basic_sendable_event_trait::{BasicEventStatus, BasicSendableEvent};
use artcraft_client::enums::tauri::ux::tauri_event_name::TauriEventName;
use serde_derive::Serialize;

/// Flash an informational (non-error) message in the frontend — a "notice"
/// toast, e.g. "Higgsfield is checking your media for Intellectual Property
/// and Likeness". Purely informational: nothing failed and nothing is
/// required of the user.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct FlashNoticeEvent {
  pub message: String,
}

impl FlashNoticeEvent {
  pub fn new(message: impl Into<String>) -> Self {
    Self { message: message.into() }
  }
}

impl BasicSendableEvent for FlashNoticeEvent {
  const FRONTEND_EVENT_NAME: TauriEventName = TauriEventName::FlashNoticeEvent;
  const EVENT_STATUS: BasicEventStatus = BasicEventStatus::Success;
}

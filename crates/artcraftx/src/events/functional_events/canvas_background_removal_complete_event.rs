use crate::events::basic_sendable_event_trait::{BasicEventStatus, BasicSendableEvent};
use enums::tauri::ux::tauri_event_name::TauriEventName;
use serde_derive::Serialize;
use sqlite_identifiers::media_file_token::MediaFileToken;
use url::Url;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CanvasBackgroundRemovalCompleteEvent {
  pub media_token: MediaFileToken,
  pub image_cdn_url: Url,
  pub maybe_frontend_subscriber_id: Option<String>,
  pub maybe_frontend_subscriber_payload: Option<String>,
}

impl BasicSendableEvent for CanvasBackgroundRemovalCompleteEvent {
  const FRONTEND_EVENT_NAME: TauriEventName = TauriEventName::CanvasBgRemovedEvent;
  const EVENT_STATUS: BasicEventStatus = BasicEventStatus::Success;
}

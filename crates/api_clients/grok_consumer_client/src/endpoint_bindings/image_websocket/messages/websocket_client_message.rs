use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

/// Number of images requested per prompt. The web app pairs this with
/// `enable_side_by_side` to render a 2-up result (observed 2026-08-23).
const DEFAULT_NUM_GENERATIONS: usize = 2;

/// The web app requests two side-by-side images by default.
const DEFAULT_ENABLE_SIDE_BY_SIDE: bool = true;

/// "Pro"/quality mode. Off by default (fast mode).
const DEFAULT_ENABLE_PRO: bool = false;

/// The web app does not request a watermark.
const DEFAULT_ENABLE_WATERMARK: bool = false;

#[derive(Serialize)]
pub struct WebsocketClientMessage {
  /// Type of message
  /// eg. "conversation.item.create"
  pub r#type: String,

  /// Unix timestamp
  pub timestamp: u64,

  pub item: ClientMessageItem,
}

#[derive(Serialize)]
pub struct ClientMessageItem {
  /// Type of item
  /// eg. "message"
  pub r#type: String,

  pub content: Vec<ClientMessageItemContent>,
}

#[derive(Serialize)]
pub struct ClientMessageItemContent {
  /// UUID
  #[serde(rename="requestId")]
  pub request_id: String,

  /// Type of item content
  /// eg. "input_text"
  pub r#type: String,

  /// The prompt for the request
  pub text: String,

  pub properties: ClientMessageItemContentProperties,
}

/// Field order mirrors the web app's wire payload (observed 2026-08-23).
#[derive(Serialize)]
pub struct ClientMessageItemContentProperties {
  pub section_count: usize,
  pub is_kids_mode: bool,
  pub enable_nsfw: bool,
  pub skip_upsampler: bool,
  pub enable_side_by_side: bool,
  pub is_initial: bool,
  pub aspect_ratio: ClientMessageAspectRatio,
  pub enable_pro: bool,
  pub num_generations: usize,
  pub enable_watermark: bool,
}

#[derive(Serialize, Clone, Copy, Debug)]
pub enum ClientMessageAspectRatio {
  #[serde(rename = "2:3")]
  TallTwoByThree,

  #[serde(rename = "3:2")]
  WideThreeByTwo,

  #[serde(rename = "16:9")]
  WideSixteenByNine,

  #[serde(rename = "1:1")]
  Square,
}

impl WebsocketClientMessage {
  /// Create a new image prompt websocket client message.
  ///
  /// Mirrors the web app's `conversation.item.create` send frame (see
  /// `external/requests/sites/grok.com/2026-08-23-imagine/06_websocket_after_image_gen.har.json`).
  pub fn new_image_prompt(prompt: &str, aspect_ratio: ClientMessageAspectRatio) -> Self {
    Self {
      r#type: "conversation.item.create".to_string(),
      timestamp: Utc::now().timestamp_millis() as u64,
      item: ClientMessageItem {
        r#type: "message".to_string(),
        content: vec![
          ClientMessageItemContent {
            request_id: Uuid::new_v4().to_string(),
            r#type: "input_text".to_string(),
            text: prompt.to_string(),
            properties: ClientMessageItemContentProperties {
              section_count: 0,
              is_kids_mode: false,
              enable_nsfw: true,
              skip_upsampler: false,
              enable_side_by_side: DEFAULT_ENABLE_SIDE_BY_SIDE,
              is_initial: false,
              aspect_ratio,
              enable_pro: DEFAULT_ENABLE_PRO,
              num_generations: DEFAULT_NUM_GENERATIONS,
              enable_watermark: DEFAULT_ENABLE_WATERMARK,
            },
          }
        ],
      },
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // The web app's real send frame (from 06_websocket_after_image_gen.har.json),
  // minus the per-request `requestId` and `timestamp`, which we generate.
  #[test]
  fn image_prompt_wire_format_matches_capture() {
    let message = WebsocketClientMessage::new_image_prompt(
      "A dead tree stump in the middle of a forest meadow",
      ClientMessageAspectRatio::TallTwoByThree,
    );

    let value = serde_json::to_value(&message).unwrap();

    assert_eq!(value["type"], "conversation.item.create");
    assert_eq!(value["item"]["type"], "message");

    let content = &value["item"]["content"][0];
    assert_eq!(content["type"], "input_text");
    assert_eq!(content["text"], "A dead tree stump in the middle of a forest meadow");
    assert!(content["requestId"].as_str().is_some());

    let properties = &content["properties"];
    assert_eq!(properties["section_count"], 0);
    assert_eq!(properties["is_kids_mode"], false);
    assert_eq!(properties["enable_nsfw"], true);
    assert_eq!(properties["skip_upsampler"], false);
    assert_eq!(properties["enable_side_by_side"], true);
    assert_eq!(properties["is_initial"], false);
    assert_eq!(properties["aspect_ratio"], "2:3");
    assert_eq!(properties["enable_pro"], false);
    assert_eq!(properties["num_generations"], 2);
    assert_eq!(properties["enable_watermark"], false);
  }

  #[test]
  fn aspect_ratios_serialize_to_grok_strings() {
    let ratio = |r| serde_json::to_value(r).unwrap();
    assert_eq!(ratio(ClientMessageAspectRatio::TallTwoByThree), "2:3");
    assert_eq!(ratio(ClientMessageAspectRatio::WideThreeByTwo), "3:2");
    assert_eq!(ratio(ClientMessageAspectRatio::WideSixteenByNine), "16:9");
    assert_eq!(ratio(ClientMessageAspectRatio::Square), "1:1");
  }
}

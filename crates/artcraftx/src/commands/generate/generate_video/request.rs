use router::api::router_aspect_ratio::RouterAspectRatio;
use router::api::router_resolution::RouterResolution;
use sqlite_identifiers::enums::generation_provider::GenerationProvider;
use sqlite_identifiers::enums::tauri_command_caller::TauriCommandCaller;
use serde_derive::{Deserialize, Serialize};
use artcraft_tokens::tokens::characters::CharacterToken;
use sqlite_identifiers::ids::media_file_token::MediaFileToken;

use crate::commands::utils::response::success_response_wrapper::SerializeMarker;

/// This is used in the Tauri command bridge.
/// Don't change the serializations without coordinating with the frontend.
///
/// MIGRATION (2026-07): we're moving the frontend to send storyteller-web omni
/// identifiers (`CommonVideoModel` serde strings). Variants whose legacy Tauri
/// id differs carry the omni id as a `#[serde(alias)]` so BOTH deserialize.
/// Once the frontend is 100% omni ids, the legacy renames can be dropped.
#[derive(Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TauriVideoModel {
  #[serde(rename = "grok_video", alias = "grok_imagine_video")]
  GrokVideo,

  #[serde(rename = "grok_imagine_video_1p5")]
  GrokImagineVideo1p5,

  #[serde(rename = "kling_1.6_pro", alias = "kling_1p6_pro")]
  Kling16Pro,

  #[serde(rename = "kling_2.1_pro", alias = "kling_2p1_pro")]
  Kling21Pro,

  #[serde(rename = "kling_2.1_master", alias = "kling_2p1_master")]
  Kling21Master,

  #[serde(rename = "kling_2p5_turbo_pro")]
  Kling2p5TurboPro,

  #[serde(rename = "kling_2p6_pro")]
  Kling2p6Pro,

  #[serde(rename = "kling_3p0_standard")]
  Kling3p0Standard,

  #[serde(rename = "kling_3p0_pro")]
  Kling3p0Pro,

  #[serde(rename = "happy_horse_1p0")]
  HappyHorse1p0,

  #[serde(rename = "seedance_1.0_lite", alias = "seedance_1p0_lite")]
  Seedance10Lite,

  #[serde(rename = "seedance_1p5_pro")]
  Seedance1p5Pro,

  #[serde(rename = "seedance_2p0")]
  Seedance2p0,

  #[serde(rename = "seedance_2p0_fast")]
  Seedance2p0Fast,

  #[serde(rename = "sora_2")]
  Sora2,

  #[serde(rename = "sora_2_pro")]
  Sora2Pro,

  #[serde(rename = "veo_2")]
  Veo2,

  #[serde(rename = "veo_3")]
  Veo3,

  #[serde(rename = "veo_3_fast")]
  Veo3Fast,

  #[serde(rename = "veo_3p1")]
  Veo3p1,

  #[serde(rename = "veo_3p1_fast")]
  Veo3p1Fast,

  #[serde(rename = "veo_3p1_lite")]
  Veo3p1Lite,

  #[serde(rename = "vidu_q3")]
  ViduQ3,

  #[serde(rename = "vidu_q3_turbo")]
  ViduQ3Turbo,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct TauriGenerateVideoRequest {
  /// Stable id (`credential_{entropy}`) of the stored credential (account)
  /// to generate with. Loaded from disk; generation routes to the
  /// credential's service.
  pub credential_id: Option<String>,

  pub provider: Option<GenerationProvider>,
  pub model: Option<TauriVideoModel>,

  pub prompt: Option<String>,
  pub negative_prompt: Option<String>,

  #[deprecated(note = "Use start_frame_image_media_token instead")]
  pub image_media_token: Option<MediaFileToken>,

  pub start_frame_image_media_token: Option<MediaFileToken>,
  pub end_frame_image_media_token: Option<MediaFileToken>,

  pub reference_image_media_tokens: Option<Vec<MediaFileToken>>,
  pub reference_video_media_tokens: Option<Vec<MediaFileToken>>,
  pub reference_audio_media_tokens: Option<Vec<MediaFileToken>>,
  pub reference_character_tokens: Option<Vec<CharacterToken>>,

  pub aspect_ratio: Option<RouterAspectRatio>,
  pub resolution: Option<RouterResolution>,

  pub duration_seconds: Option<u16>,
  pub generate_audio: Option<bool>,
  pub video_batch_count: Option<u16>,

  #[deprecated(note = "Use start_frame_image_media_token instead")]
  pub sora_orientation: Option<SoraOrientation>,

  #[deprecated(note = "Use start_frame_image_media_token instead")]
  pub grok_aspect_ratio: Option<GrokAspectRatio>,

  pub frontend_caller: Option<TauriCommandCaller>,
  pub frontend_subscriber_id: Option<String>,
  pub frontend_subscriber_payload: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum SoraOrientation {
  Portrait,
  Landscape,
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum GrokAspectRatio {
  Portrait,
  Landscape,
  Square,
}

#[derive(Serialize)]
pub struct TauriGenerateVideoResponse {
}

impl SerializeMarker for TauriGenerateVideoResponse {}

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum TauriGenerateVideoErrorType {
  ModelNotSpecified,
  NoProviderAvailable,
  ServerError,
  NeedsFalApiKey,
  FalError,
  NeedsStorytellerCredentials,
  /// Problem with the selected account credential (absent, unknown, or
  /// unusable). The backend also flashes a dismissable modal.
  CredentialProblem,
}

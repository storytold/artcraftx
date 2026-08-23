use std::fmt::Debug;
use std::sync::Arc;

use artcraft_client::tokens::generic_inference_jobs::InferenceJobToken;

#[derive(Clone, Debug)]
pub struct ArtcraftImageResponsePayload {
  pub inference_job_token: InferenceJobToken,
}

#[derive(Clone, Debug)]
pub struct FalImageResponsePayload {
  pub request_id: Option<String>,
  pub gateway_request_id: Option<String>,

  /// The queue status URL (for polling job progress).
  pub maybe_status_url: Option<String>,

  /// The queue response URL (for fetching completed results).
  pub maybe_response_url: Option<String>,

  /// The outbound request that was sent to Fal.
  /// Stored as a trait object so any Request type can be captured.
  /// Use `format!("{:?}", ...)` or `format!("{:#?}", ...)` to print.
  pub maybe_outbound_request: Option<Arc<dyn Debug + Send + Sync>>,
}

/// Response from the Seedance2Pro/Kinovi provider (used for Midjourney
/// image generation). Mirrors `Seedance2proVideoResponsePayload` on the
/// video side.
#[derive(Clone, Debug)]
pub struct Seedance2proImageResponsePayload {
  pub order_id: String,
  pub task_id: String,
  pub maybe_order_ids: Option<Vec<String>>,
  pub maybe_task_ids: Option<Vec<String>>,
}

/// Response from the first-party (cookie-session) Midjourney provider. The
/// `job_id` is Midjourney's own job id, used as the task `provider_job_id` and
/// as the websocket `subscribe_to_job` key.
#[derive(Clone, Debug)]
pub struct MidjourneyImageResponsePayload {
  pub job_id: String,
}

#[derive(Clone, Debug)]
pub enum GenerateImageResponse {
  Artcraft(ArtcraftImageResponsePayload),
  Fal(FalImageResponsePayload),
  Midjourney(MidjourneyImageResponsePayload),
  Seedance2Pro(Seedance2proImageResponsePayload),
}

impl GenerateImageResponse {
  pub fn get_artcraft_payload(&self) -> Option<ArtcraftImageResponsePayload> {
    match self {
      Self::Artcraft(p) => Some(p.clone()),
      _ => None,
    }
  }

  pub fn get_fal_payload(&self) -> Option<FalImageResponsePayload> {
    match self {
      Self::Fal(p) => Some(p.clone()),
      _ => None,
    }
  }

  pub fn get_midjourney_payload(&self) -> Option<MidjourneyImageResponsePayload> {
    match self {
      Self::Midjourney(p) => Some(p.clone()),
      _ => None,
    }
  }

  pub fn get_seedance2pro_payload(&self) -> Option<Seedance2proImageResponsePayload> {
    match self {
      Self::Seedance2Pro(p) => Some(p.clone()),
      _ => None,
    }
  }
}

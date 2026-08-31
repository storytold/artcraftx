//! POST `/fnf/jobs/v2/seedance_2_0_mini` — enqueue a Seedance 2.0 Mini video
//! job (job set type `seedance_2_0_mini`).
//!
//! Options read off the web app on 2026-08-31: duration 4–15 s, aspect
//! Auto / 16:9 / 9:16 / 4:3 / 3:4 / 1:1 / 21:9, 480p / 720p, audio on/off,
//! uploaded media. No bitrate control.

use crate::client::higgsfield_host::HiggsfieldHost;
use crate::client::send_request::{send_json_request, HttpMethod};
use crate::credentials::higgsfield_auth::HiggsfieldAuth;
use crate::error::higgsfield_client_error::HiggsfieldClientError;
use crate::error::higgsfield_error::HiggsfieldError;
use crate::types::enqueue_jobs_response::EnqueueJobsResponse;
use crate::types::image_aspect_ratio::ImageAspectRatio;
use crate::types::image_batch_size::ImageBatchSize;
use crate::types::video_aspect_ratio::SeedanceVideoAspectRatio;
use crate::types::video_dimensions::VideoDimensions;
use crate::types::video_duration::{VideoDurationRange, VideoDurationSeconds};
use crate::types::video_resolution::VideoResolution;
use serde::Serialize;

const PATH: &str = "/fnf/jobs/v2/seedance_2_0_mini";

/// The `model` field the web app sends on this endpoint.
const MODEL: &str = "seedance_2_0_mini";

/// The resolution tiers the web app offers for Seedance 2.0 Mini.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Seedance2p0MiniResolution {
  #[default]
  P480,
  P720,
}

impl Seedance2p0MiniResolution {
  pub fn all() -> [Self; 2] {
    [Self::P480, Self::P720]
  }

  pub fn to_video_resolution(self) -> VideoResolution {
    match self {
      Self::P480 => VideoResolution::P480,
      Self::P720 => VideoResolution::P720,
    }
  }
}

impl Serialize for Seedance2p0MiniResolution {
  fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    self.to_video_resolution().serialize(serializer)
  }
}

pub struct Seedance2p0MiniArgs<'a> {
  pub request: Seedance2p0MiniRequest,
  pub auth: &'a HiggsfieldAuth,
  pub host: &'a HiggsfieldHost,
}

/// The material part of a Seedance 2.0 Mini request. Serializable so it
/// can be logged or persisted separately from the session.
#[derive(Clone, Debug, Serialize)]
pub struct Seedance2p0MiniRequest {
  pub prompt: String,

  /// `Auto` follows the reference media; for text-to-video the web app
  /// sends it as `16:9`, and so do we.
  pub aspect_ratio: SeedanceVideoAspectRatio,

  pub resolution: Seedance2p0MiniResolution,

  /// Clip length; see [`Self::DURATION`].
  pub duration: VideoDurationSeconds,

  /// How many clips to generate (1–4). Each costs credits.
  pub batch_size: ImageBatchSize,

  /// Generate a soundtrack (the web app's audio "On").
  pub generate_audio: bool,

  /// Reference media ids/URLs as the web app sends them. Empty for
  /// text-to-video.
  pub medias: Vec<String>,

  /// Spend from the plan's "unlimited" pool instead of credits, if the plan
  /// has one.
  pub use_unlim: bool,

  /// Override the pixel size sent with the request. When `None`, derived
  /// from `aspect_ratio` + `resolution` the way the web app does.
  pub maybe_dimensions: Option<VideoDimensions>,
}

impl Seedance2p0MiniRequest {
  /// The web app's duration slider range.
  pub const DURATION: VideoDurationRange = VideoDurationRange::new(4, 15);

  /// A text-to-video request with the web app's defaults (1 clip, audio on,
  /// credits).
  pub fn text_to_video(prompt: impl Into<String>, aspect_ratio: SeedanceVideoAspectRatio, resolution: Seedance2p0MiniResolution, duration: VideoDurationSeconds) -> Self {
    Self {
      prompt: prompt.into(),
      aspect_ratio,
      resolution,
      duration,
      batch_size: ImageBatchSize::One,
      generate_audio: true,
      medias: Vec::new(),
      use_unlim: false,
      maybe_dimensions: None,
    }
  }

  fn validate(&self) -> Result<(), HiggsfieldClientError> {
    if self.prompt.trim().is_empty() {
      return Err(HiggsfieldClientError::InvalidRequest("prompt is empty".to_string()));
    }
    Self::DURATION.validate(self.duration)
  }

  fn wire_aspect_ratio(&self) -> ImageAspectRatio {
    match self.aspect_ratio {
      SeedanceVideoAspectRatio::Auto if self.medias.is_empty() => ImageAspectRatio::Landscape16x9,
      other => other.to_image_aspect_ratio(),
    }
  }

  fn dimensions(&self) -> Result<VideoDimensions, HiggsfieldClientError> {
    if let Some(dimensions) = self.maybe_dimensions {
      return Ok(dimensions);
    }
    VideoDimensions::for_aspect_ratio(&self.wire_aspect_ratio(), &self.resolution.to_video_resolution())
        .ok_or_else(|| HiggsfieldClientError::InvalidRequest("can't derive video dimensions; pass maybe_dimensions".to_string()))
  }

  fn to_body(&self) -> Result<Seedance2p0MiniRequestBody, HiggsfieldClientError> {
    let dimensions = self.dimensions()?;
    Ok(Seedance2p0MiniRequestBody {
      params: Seedance2p0MiniParams {
        prompt: self.prompt.clone(),
        batch_size: self.batch_size,
        duration: self.duration,
        generate_audio: self.generate_audio,
        model: MODEL,
        aspect_ratio: self.wire_aspect_ratio(),
        resolution: self.resolution,
        width: dimensions.width,
        height: dimensions.height,
        medias: self.medias.clone(),
      },
      use_unlim: self.use_unlim,
      use_free_gens: false,
    })
  }
}

/// Enqueue the job. The response's job ids are what to poll (see
/// `endpoints::jobs`); a finished job's `results.raw.url` is the `.mp4`.
pub async fn seedance_2p0_mini(args: Seedance2p0MiniArgs<'_>) -> Result<EnqueueJobsResponse, HiggsfieldError> {
  args.request.validate()?;
  let body = args.request.to_body()?;
  send_json_request(HttpMethod::Post, PATH, args.auth, args.host, Some(&body)).await
}

// ── Wire format ──

#[derive(Serialize)]
struct Seedance2p0MiniRequestBody {
  params: Seedance2p0MiniParams,
  use_unlim: bool,
  use_free_gens: bool,
}

#[derive(Serialize)]
struct Seedance2p0MiniParams {
  prompt: String,
  batch_size: ImageBatchSize,
  duration: VideoDurationSeconds,
  generate_audio: bool,
  model: &'static str,
  aspect_ratio: ImageAspectRatio,
  resolution: Seedance2p0MiniResolution,
  width: u32,
  height: u32,
  medias: Vec<String>,
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::endpoints::generate::video::test_fixtures::enqueue_response;
  use crate::types::job_set_type::JobSetType;
  use serde_json::Value;

  const SERVER_PARAMS: &str = r#"{"width":854,"height":480,"prompt":"a shiba inu skateboarding down a hill","genre":"auto","medias":[],"duration":4,"resolution":"480p","aspect_ratio":"16:9","generate_audio":true,"multi_shots":false,"multi_shot_mode":"custom","multi_prompt":[],"speedramp":"auto","reference_elements":[],"prompt_language":"en"}"#;

  #[test]
  fn resolutions_match_the_web_app_menu() {
    let wire: Vec<String> = Seedance2p0MiniResolution::all().iter().map(|r| r.to_video_resolution().to_string()).collect();
    assert_eq!(wire, ["480p", "720p"]);
    assert_eq!(Seedance2p0MiniRequest::DURATION, VideoDurationRange::new(4, 15));
  }

  #[test]
  fn wire_body_matches_captured_request() {
    let request = Seedance2p0MiniRequest::text_to_video("a shiba inu skateboarding down a hill", SeedanceVideoAspectRatio::Auto, Seedance2p0MiniResolution::P480, VideoDurationSeconds::new(4));
    let actual: Value = serde_json::to_value(request.to_body().unwrap()).unwrap();
    let expected: Value = serde_json::from_str(r#"{"params":{"prompt":"a shiba inu skateboarding down a hill","batch_size":1,"duration":4,"generate_audio":true,"model":"seedance_2_0_mini","aspect_ratio":"16:9","resolution":"480p","width":854,"height":480,"medias":[]},"use_unlim":false,"use_free_gens":false}"#).unwrap();
    assert_eq!(actual, expected);
  }

  #[test]
  fn validation() {
    let bad = Seedance2p0MiniRequest::text_to_video("p", SeedanceVideoAspectRatio::Square1x1, Seedance2p0MiniResolution::P720, VideoDurationSeconds::new(3));
    assert!(matches!(bad.validate(), Err(HiggsfieldClientError::InvalidRequest(_))));
  }

  #[tokio::test]
  async fn invalid_request_fails_before_any_http() {
    let auth = HiggsfieldAuth::new("token");
    let host = HiggsfieldHost::Custom("http://127.0.0.1:9".to_string());
    let request = Seedance2p0MiniRequest::text_to_video("", SeedanceVideoAspectRatio::Square1x1, Seedance2p0MiniResolution::P480, VideoDurationSeconds::new(4));
    let err = seedance_2p0_mini(Seedance2p0MiniArgs { request, auth: &auth, host: &host }).await.unwrap_err();
    assert!(matches!(err, HiggsfieldError::Client(HiggsfieldClientError::InvalidRequest(_))));
  }

  #[test]
  fn enqueue_response_parses() {
    let response: EnqueueJobsResponse = serde_json::from_str(&enqueue_response("seedance_2_0_mini", 400, SERVER_PARAMS, true)).unwrap();
    let job_set = response.first_job_set().unwrap();
    assert_eq!(job_set.job_set_type, JobSetType::Seedance2p0Mini);
    assert_eq!(job_set.cost, Some(400.0));
    assert!(job_set.params.bitrate_mode.is_none());
  }

  // ── Live (ignored: needs a real session and spends credits) ──

  #[tokio::test]
  #[ignore]
  async fn live_enqueue_seedance_2p0_mini_from_app_credential_and_poll() -> anyhow::Result<()> {
    use crate::test_utils::higgsfield_credential_toml::load_higgsfield_session_from_app_credential;
    use crate::test_utils::poll_job_to_completion::poll_job_to_completion;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();

    let session = load_higgsfield_session_from_app_credential()?;
    let auth = session.auth().await.map_err(|err| anyhow::anyhow!("{err}"))?;
    let request = Seedance2p0MiniRequest::text_to_video("a shiba inu dunking a basketball", SeedanceVideoAspectRatio::Landscape16x9, Seedance2p0MiniResolution::P480, Seedance2p0MiniRequest::DURATION.shortest());
    println!("\n===== request =====\n{:#?}", request);
    let response = seedance_2p0_mini(Seedance2p0MiniArgs { request, auth: &auth, host: &HiggsfieldHost::Higgsfield }).await.map_err(|err| anyhow::anyhow!("{err}"))?;
    println!("\n===== POST {PATH} =====\n{:#?}", response);
    assert_eq!(response.first_job_set().unwrap().job_set_type, JobSetType::Seedance2p0Mini);
    let job = poll_job_to_completion(&session, &response.job_ids()[0]).await?;
    assert!(job.result_url().unwrap().ends_with(".mp4"));
    Ok(())
  }
}

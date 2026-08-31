//! POST `/fnf/jobs/v2/gpt_image_2` — enqueue a GPT Image 2 job (the web
//! app's "GPT Image 2"; job set type `gpt_image_2`).

use crate::client::higgsfield_host::HiggsfieldHost;
use crate::client::send_request::{send_json_request, HttpMethod};
use crate::credentials::higgsfield_auth::HiggsfieldAuth;
use crate::error::higgsfield_client_error::HiggsfieldClientError;
use crate::error::higgsfield_error::HiggsfieldError;
use crate::types::enqueue_jobs_response::EnqueueJobsResponse;
use crate::types::gpt_image_quality::GptImageQuality;
use crate::types::image_aspect_ratio::ImageAspectRatio;
use crate::types::image_dimensions::ImageDimensions;
use crate::types::image_resolution::ImageResolution;
use serde::Serialize;
use crate::types::string_enum::string_enum;

const PATH: &str = "/fnf/jobs/v2/gpt_image_2";

/// The `model` field the web app sends on this endpoint.
const MODEL: &str = "gpt_image_2";

/// Batch sizes the web app offers.
const MIN_BATCH_SIZE: u32 = 1;
const MAX_BATCH_SIZE: u32 = 4;

string_enum! {
  /// The backend variant behind GPT Image 2. The web app currently always
  /// sends `videotape-alpha`; it's echoed back as `params.model`.
  GptImage2SubModel {
    VideotapeAlpha => "videotape-alpha",
  }
}

impl Default for GptImage2SubModel {
  fn default() -> Self {
    Self::VideotapeAlpha
  }
}

pub struct GptImage2Args<'a> {
  pub request: GptImage2Request,
  pub auth: &'a HiggsfieldAuth,
  pub host: &'a HiggsfieldHost,
}

/// The material part of a GPT Image 2 request. Serializable so it can be
/// logged or persisted separately from the session.
#[derive(Clone, Debug, Serialize)]
pub struct GptImage2Request {
  pub prompt: String,

  pub aspect_ratio: ImageAspectRatio,

  pub quality: GptImageQuality,

  pub resolution: ImageResolution,

  /// How many images to generate (1–4). Each costs credits.
  pub batch_size: u32,

  /// Reference media URLs (image-to-image). Empty for text-to-image.
  pub medias: Vec<String>,

  pub sub_model: GptImage2SubModel,

  /// Spend from the plan's "unlimited" pool instead of credits, if the plan
  /// has one.
  pub use_unlim: bool,

  /// Override the pixel size sent with the request. When `None`, the size
  /// the web app would send for `aspect_ratio` + `resolution` is used.
  pub maybe_dimensions: Option<ImageDimensions>,
}

impl GptImage2Request {
  /// A text-to-image request with the web app's defaults (1 image, credits).
  pub fn text_to_image(
    prompt: impl Into<String>,
    aspect_ratio: ImageAspectRatio,
    quality: GptImageQuality,
    resolution: ImageResolution,
  ) -> Self {
    Self {
      prompt: prompt.into(),
      aspect_ratio,
      quality,
      resolution,
      batch_size: 1,
      medias: Vec::new(),
      sub_model: GptImage2SubModel::default(),
      use_unlim: false,
      maybe_dimensions: None,
    }
  }

  fn validate(&self) -> Result<(), HiggsfieldClientError> {
    if self.prompt.trim().is_empty() {
      return Err(HiggsfieldClientError::InvalidRequest("prompt is empty".to_string()));
    }
    if !(MIN_BATCH_SIZE..=MAX_BATCH_SIZE).contains(&self.batch_size) {
      return Err(HiggsfieldClientError::InvalidRequest(format!(
        "batch_size must be between {MIN_BATCH_SIZE} and {MAX_BATCH_SIZE}, got {}", self.batch_size,
      )));
    }
    Ok(())
  }

  fn dimensions(&self) -> Result<ImageDimensions, HiggsfieldClientError> {
    if let Some(dimensions) = self.maybe_dimensions {
      return Ok(dimensions);
    }
    ImageDimensions::for_aspect_ratio(&self.aspect_ratio, &self.resolution)
        .ok_or_else(|| HiggsfieldClientError::InvalidRequest(format!(
          "can't derive dimensions for aspect ratio {} at {}; pass maybe_dimensions",
          self.aspect_ratio, self.resolution,
        )))
  }
}

/// Enqueue the job. The response's job ids are what to poll (see
/// `endpoints::jobs`).
pub async fn gpt_image_2(args: GptImage2Args<'_>) -> Result<EnqueueJobsResponse, HiggsfieldError> {
  let request = args.request;
  request.validate()?;
  let dimensions = request.dimensions()?;

  let body = GptImage2RequestBody {
    params: GptImage2Params {
      prompt: request.prompt,
      aspect_ratio: request.aspect_ratio,
      quality: request.quality,
      resolution: request.resolution,
      sub_model: request.sub_model,
      batch_size: request.batch_size,
      model: MODEL,
      width: dimensions.width,
      height: dimensions.height,
      medias: request.medias,
    },
    use_unlim: request.use_unlim,
  };

  send_json_request(HttpMethod::Post, PATH, args.auth, args.host, Some(&body)).await
}

// ── Wire format ──

#[derive(Serialize)]
struct GptImage2RequestBody {
  params: GptImage2Params,
  use_unlim: bool,
}

#[derive(Serialize)]
struct GptImage2Params {
  prompt: String,
  aspect_ratio: ImageAspectRatio,
  quality: GptImageQuality,
  resolution: ImageResolution,
  sub_model: GptImage2SubModel,
  batch_size: u32,
  model: &'static str,
  width: u32,
  height: u32,
  medias: Vec<String>,
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::{json, Value};

  // ── Outbound shape ──

  #[test]
  fn wire_body_matches_captured_request() {
    // Captured from the web app: 9:16 at 2k, high quality, one image.
    let request = GptImage2Request::text_to_image("a corgi on a bike", ImageAspectRatio::Portrait9x16, GptImageQuality::High, ImageResolution::TwoK);
    let dimensions = request.dimensions().unwrap();
    let body = GptImage2RequestBody {
      params: GptImage2Params {
        prompt: request.prompt,
        aspect_ratio: request.aspect_ratio,
        quality: request.quality,
        resolution: request.resolution,
        sub_model: request.sub_model,
        batch_size: request.batch_size,
        model: MODEL,
        width: dimensions.width,
        height: dimensions.height,
        medias: vec![],
      },
      use_unlim: false,
    };

    let actual: Value = serde_json::to_value(&body).unwrap();
    let expected: Value = serde_json::from_str(r#"{"params":{"prompt":"a corgi on a bike","aspect_ratio":"9:16","quality":"high","resolution":"2k","sub_model":"videotape-alpha","batch_size":1,"model":"gpt_image_2","width":1152,"height":2048,"medias":[]},"use_unlim":false}"#).unwrap();
    assert_eq!(actual, expected);
  }

  #[test]
  fn public_request_serializes_with_typed_enums() {
    let request = GptImage2Request::text_to_image("a cat", ImageAspectRatio::Square1x1, GptImageQuality::Medium, ImageResolution::OneK);
    let value = serde_json::to_value(&request).unwrap();
    assert_eq!(value["quality"], json!("medium"));
    assert_eq!(value["sub_model"], json!("videotape-alpha"));
    assert_eq!(value["aspect_ratio"], json!("1:1"));
  }

  // ── Validation ──

  #[test]
  fn empty_prompt_is_rejected() {
    let request = GptImage2Request::text_to_image("", ImageAspectRatio::Square1x1, GptImageQuality::Low, ImageResolution::OneK);
    assert!(matches!(request.validate(), Err(HiggsfieldClientError::InvalidRequest(_))));
  }

  #[test]
  fn batch_size_out_of_range_is_rejected() {
    let mut request = GptImage2Request::text_to_image("p", ImageAspectRatio::Square1x1, GptImageQuality::Low, ImageResolution::OneK);
    request.batch_size = 0;
    assert!(matches!(request.validate(), Err(HiggsfieldClientError::InvalidRequest(_))));
  }

  #[tokio::test]
  async fn invalid_request_fails_before_any_http() {
    let auth = HiggsfieldAuth::new("token");
    let host = HiggsfieldHost::Custom("http://127.0.0.1:9".to_string());
    let request = GptImage2Request::text_to_image(" ", ImageAspectRatio::Square1x1, GptImageQuality::Low, ImageResolution::OneK);

    let err = gpt_image_2(GptImage2Args { request, auth: &auth, host: &host }).await.unwrap_err();
    assert!(matches!(err, HiggsfieldError::Client(HiggsfieldClientError::InvalidRequest(_))));
  }

  // ── Inbound shape ──

  #[test]
  fn enqueue_response_parses() {
    use crate::types::enqueue_jobs_response::tests::GPT_IMAGE_ENQUEUE_RESPONSE;
    use crate::types::job_set_type::JobSetType;
    let response: EnqueueJobsResponse = serde_json::from_str(GPT_IMAGE_ENQUEUE_RESPONSE).unwrap();
    assert_eq!(response.first_job_set().unwrap().job_set_type, JobSetType::GptImage2);
  }

  // ── Live (ignored: needs a real session and spends credits) ──

  /// Enqueues a GPT Image 2 job off the desktop app's saved Higgsfield
  /// login (`~/Artcraft/artcraftx/credentials/higgsfield_cookies.toml`),
  /// prints the enqueue response, then follows the job through the status
  /// endpoints until it completes. Cheapest settings (1 image, 1k, low).
  #[tokio::test]
  #[ignore]
  async fn live_enqueue_gpt_image_2_from_app_credential_and_poll() -> anyhow::Result<()> {
    use crate::test_utils::higgsfield_credential_toml::load_higgsfield_session_from_app_credential;
    use crate::test_utils::poll_job_to_completion::poll_job_to_completion;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();

    let session = load_higgsfield_session_from_app_credential()?;
    let auth = session.auth().await.map_err(|err| anyhow::anyhow!("minting a session token failed: {err}"))?;

    let request = GptImage2Request::text_to_image("a corgi on a bike", ImageAspectRatio::Square1x1, GptImageQuality::Low, ImageResolution::OneK);
    println!("\n===== request =====\n{:#?}", request);

    let response = gpt_image_2(GptImage2Args {
      request,
      auth: &auth,
      host: &HiggsfieldHost::Higgsfield,
    }).await.map_err(|err| anyhow::anyhow!("{err}"))?;

    println!("\n===== POST /fnf/jobs/v2/gpt_image_2 =====\n{:#?}", response);
    let job_ids = response.job_ids();
    println!("job ids: {:?}", job_ids);
    assert_eq!(job_ids.len(), 1);
    assert_eq!(response.first_job_set().unwrap().job_set_type, crate::types::job_set_type::JobSetType::GptImage2);

    let job = poll_job_to_completion(&session, &job_ids[0]).await?;
    assert!(job.result_url().is_some(), "completed job should have a result url");
    Ok(())
  }

  #[tokio::test]
  #[ignore]
  async fn live_enqueue_gpt_image_2() -> anyhow::Result<()> {
    use crate::test_utils::higgsfield_test_secrets::load_higgsfield_test_auth;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();

    let auth = load_higgsfield_test_auth().await?;
    let response = gpt_image_2(GptImage2Args {
      request: GptImage2Request::text_to_image("a corgi on a bike", ImageAspectRatio::Portrait9x16, GptImageQuality::High, ImageResolution::TwoK),
      auth: &auth,
      host: &HiggsfieldHost::Higgsfield,
    }).await.map_err(|err| anyhow::anyhow!("{err}"))?;

    println!("Enqueued job ids: {:?}", response.job_ids());
    println!("Wallet: {:?}", response.wallet);
    assert!(!response.job_ids().is_empty());
    Ok(())
  }
}

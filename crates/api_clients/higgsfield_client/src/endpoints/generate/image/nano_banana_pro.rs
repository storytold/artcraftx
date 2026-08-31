//! POST `/fnf/jobs/nano-banana-2` — enqueue a Nano Banana Pro image job.
//!
//! "Nano Banana Pro" is the web app's name for the pipeline the API calls
//! `nano-banana-2` (job set type `nano_banana_2`).

use crate::client::higgsfield_host::HiggsfieldHost;
use crate::client::send_request::{send_json_request, HttpMethod};
use crate::credentials::higgsfield_auth::HiggsfieldAuth;
use crate::error::higgsfield_client_error::HiggsfieldClientError;
use crate::error::higgsfield_error::HiggsfieldError;
use crate::types::enqueue_jobs_response::EnqueueJobsResponse;
use crate::types::image_aspect_ratio::ImageAspectRatio;
use crate::types::image_dimensions::ImageDimensions;
use crate::types::image_resolution::ImageResolution;
use serde::Serialize;

const PATH: &str = "/fnf/jobs/nano-banana-2";

/// Batch sizes the web app offers.
const MIN_BATCH_SIZE: u32 = 1;
const MAX_BATCH_SIZE: u32 = 4;

pub struct NanoBananaProArgs<'a> {
  pub request: NanoBananaProRequest,
  pub auth: &'a HiggsfieldAuth,
  pub host: &'a HiggsfieldHost,
}

/// The material part of a Nano Banana Pro request. Serializable so it can be
/// logged or persisted separately from the session.
#[derive(Clone, Debug, Serialize)]
pub struct NanoBananaProRequest {
  pub prompt: String,

  pub aspect_ratio: ImageAspectRatio,

  pub resolution: ImageResolution,

  /// How many images to generate (1–4). Each costs credits.
  pub batch_size: u32,

  /// Reference image URLs (image-to-image). Empty for text-to-image.
  pub input_images: Vec<String>,

  /// Spend from the plan's "unlimited" pool instead of credits, if the plan
  /// has one.
  pub use_unlim: bool,

  /// Override the pixel size sent with the request. When `None`, the size
  /// the web app would send for `aspect_ratio` + `resolution` is used.
  pub maybe_dimensions: Option<ImageDimensions>,
}

impl NanoBananaProRequest {
  /// A text-to-image request with the web app's defaults (1 image, credits).
  pub fn text_to_image(prompt: impl Into<String>, aspect_ratio: ImageAspectRatio, resolution: ImageResolution) -> Self {
    Self {
      prompt: prompt.into(),
      aspect_ratio,
      resolution,
      batch_size: 1,
      input_images: Vec::new(),
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
pub async fn nano_banana_pro(args: NanoBananaProArgs<'_>) -> Result<EnqueueJobsResponse, HiggsfieldError> {
  let request = args.request;
  request.validate()?;
  let dimensions = request.dimensions()?;

  let body = NanoBananaProRequestBody {
    params: NanoBananaProParams {
      prompt: request.prompt,
      aspect_ratio: request.aspect_ratio,
      resolution: request.resolution,
      batch_size: request.batch_size,
      is_storyboard: false,
      is_zoom_control: false,
      use_unlim: request.use_unlim,
      width: dimensions.width,
      height: dimensions.height,
      input_images: request.input_images,
    },
    use_unlim: request.use_unlim,
  };

  send_json_request(HttpMethod::Post, PATH, args.auth, args.host, Some(&body)).await
}

// ── Wire format ──

#[derive(Serialize)]
struct NanoBananaProRequestBody {
  params: NanoBananaProParams,
  use_unlim: bool,
}

#[derive(Serialize)]
struct NanoBananaProParams {
  prompt: String,
  aspect_ratio: ImageAspectRatio,
  resolution: ImageResolution,
  batch_size: u32,
  is_storyboard: bool,
  is_zoom_control: bool,
  use_unlim: bool,
  width: u32,
  height: u32,
  input_images: Vec<String>,
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::{json, Value};

  // ── Outbound shape ──

  #[test]
  fn wire_body_matches_captured_request() {
    // Captured from the web app: 3:4 at 1k, one image.
    let request = NanoBananaProRequest::text_to_image("a dinosaur on a skateboard", ImageAspectRatio::Portrait3x4, ImageResolution::OneK);
    let dimensions = request.dimensions().unwrap();
    let body = NanoBananaProRequestBody {
      params: NanoBananaProParams {
        prompt: request.prompt,
        aspect_ratio: request.aspect_ratio,
        resolution: request.resolution,
        batch_size: request.batch_size,
        is_storyboard: false,
        is_zoom_control: false,
        use_unlim: false,
        width: dimensions.width,
        height: dimensions.height,
        input_images: vec![],
      },
      use_unlim: false,
    };

    let actual: Value = serde_json::to_value(&body).unwrap();
    let expected: Value = serde_json::from_str(r#"{"params":{"prompt":"a dinosaur on a skateboard","aspect_ratio":"3:4","resolution":"1k","batch_size":1,"is_storyboard":false,"is_zoom_control":false,"use_unlim":false,"width":896,"height":1200,"input_images":[]},"use_unlim":false}"#).unwrap();
    assert_eq!(actual, expected);
  }

  #[test]
  fn wire_body_4k_landscape_matches_captured_request() {
    let request = NanoBananaProRequest::text_to_image("a dinosaur on a skateboard", ImageAspectRatio::Landscape16x9, ImageResolution::FourK);
    let dimensions = request.dimensions().unwrap();
    assert_eq!((dimensions.width, dimensions.height), (5504, 3072));
  }

  #[test]
  fn explicit_dimensions_override_derived_ones() {
    let mut request = NanoBananaProRequest::text_to_image("p", ImageAspectRatio::Square1x1, ImageResolution::OneK);
    request.maybe_dimensions = Some(ImageDimensions::new(1024, 1024));
    assert_eq!(request.dimensions().unwrap(), ImageDimensions::new(1024, 1024));
  }

  #[test]
  fn public_request_serializes_with_typed_enums() {
    let request = NanoBananaProRequest::text_to_image("a cat", ImageAspectRatio::Landscape16x9, ImageResolution::TwoK);
    let value = serde_json::to_value(&request).unwrap();
    assert_eq!(value["aspect_ratio"], json!("16:9"));
    assert_eq!(value["resolution"], json!("2k"));
    assert_eq!(value["batch_size"], json!(1));
  }

  // ── Validation ──

  #[test]
  fn empty_prompt_is_rejected() {
    let request = NanoBananaProRequest::text_to_image("   ", ImageAspectRatio::Square1x1, ImageResolution::OneK);
    assert!(matches!(request.validate(), Err(HiggsfieldClientError::InvalidRequest(_))));
  }

  #[test]
  fn batch_size_out_of_range_is_rejected() {
    for batch_size in [0, 5] {
      let mut request = NanoBananaProRequest::text_to_image("p", ImageAspectRatio::Square1x1, ImageResolution::OneK);
      request.batch_size = batch_size;
      assert!(matches!(request.validate(), Err(HiggsfieldClientError::InvalidRequest(_))), "batch_size {batch_size}");
    }
  }

  #[test]
  fn unknown_aspect_ratio_without_dimensions_is_rejected() {
    let request = NanoBananaProRequest::text_to_image("p", ImageAspectRatio::Other("wide".into()), ImageResolution::OneK);
    assert!(matches!(request.dimensions(), Err(HiggsfieldClientError::InvalidRequest(_))));
  }

  #[tokio::test]
  async fn invalid_request_fails_before_any_http() {
    // No network: an unroutable custom host would fail differently.
    let auth = HiggsfieldAuth::new("token");
    let host = HiggsfieldHost::Custom("http://127.0.0.1:9".to_string());
    let mut request = NanoBananaProRequest::text_to_image("p", ImageAspectRatio::Square1x1, ImageResolution::OneK);
    request.batch_size = 99;

    let err = nano_banana_pro(NanoBananaProArgs { request, auth: &auth, host: &host }).await.unwrap_err();
    assert!(matches!(err, HiggsfieldError::Client(HiggsfieldClientError::InvalidRequest(_))));
  }

  // ── Inbound shape ──

  #[test]
  fn enqueue_response_parses() {
    use crate::types::enqueue_jobs_response::tests::NANO_BANANA_ENQUEUE_RESPONSE;
    let response: EnqueueJobsResponse = serde_json::from_str(NANO_BANANA_ENQUEUE_RESPONSE).unwrap();
    assert_eq!(response.job_ids().len(), 1);
  }

  // ── Live (ignored: needs a real session and spends credits) ──

  /// Enqueues a Nano Banana Pro job off the desktop app's saved Higgsfield
  /// login (`~/Artcraft/artcraftx/credentials/higgsfield_cookies.toml`),
  /// prints the enqueue response, then follows the job through the status
  /// endpoints until it completes. Cheapest settings (1 image, 1k).
  #[tokio::test]
  #[ignore]
  async fn live_enqueue_nano_banana_pro_from_app_credential_and_poll() -> anyhow::Result<()> {
    use crate::test_utils::higgsfield_credential_toml::load_higgsfield_session_from_app_credential;
    use crate::test_utils::poll_job_to_completion::poll_job_to_completion;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();

    let session = load_higgsfield_session_from_app_credential()?;
    let auth = session.auth().await.map_err(|err| anyhow::anyhow!("minting a session token failed: {err}"))?;

    let request = NanoBananaProRequest::text_to_image("a dinosaur on a skateboard", ImageAspectRatio::Portrait3x4, ImageResolution::OneK);
    println!("\n===== request =====\n{:#?}", request);

    let response = nano_banana_pro(NanoBananaProArgs {
      request,
      auth: &auth,
      host: &HiggsfieldHost::Higgsfield,
    }).await.map_err(|err| anyhow::anyhow!("{err}"))?;

    println!("\n===== POST /fnf/jobs/nano-banana-2 =====\n{:#?}", response);
    let job_ids = response.job_ids();
    println!("job ids: {:?}", job_ids);
    assert_eq!(job_ids.len(), 1);
    assert_eq!(response.first_job_set().unwrap().job_set_type, crate::types::job_set_type::JobSetType::NanoBanana2);

    let job = poll_job_to_completion(&session, &job_ids[0]).await?;
    assert!(job.result_url().is_some(), "completed job should have a result url");
    Ok(())
  }

  #[tokio::test]
  #[ignore]
  async fn live_enqueue_nano_banana_pro() -> anyhow::Result<()> {
    use crate::test_utils::higgsfield_test_secrets::load_higgsfield_test_auth;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();

    let auth = load_higgsfield_test_auth().await?;
    let response = nano_banana_pro(NanoBananaProArgs {
      request: NanoBananaProRequest::text_to_image("a dinosaur on a skateboard", ImageAspectRatio::Portrait3x4, ImageResolution::OneK),
      auth: &auth,
      host: &HiggsfieldHost::Higgsfield,
    }).await.map_err(|err| anyhow::anyhow!("{err}"))?;

    println!("Enqueued job ids: {:?}", response.job_ids());
    println!("Wallet: {:?}", response.wallet);
    assert!(!response.job_ids().is_empty());
    Ok(())
  }
}

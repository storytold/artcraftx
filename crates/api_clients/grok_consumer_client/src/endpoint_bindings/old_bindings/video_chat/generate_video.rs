//! `POST /rest/app-chat/conversations/new` — generate a video from an input
//! image (image-to-video). Captured 2026-08-23 in
//! `15_generate_video_from_generation.txt` (no prompt / "normal" mode) and
//! `16_generate_video_from_upload.txt` (with prompt / "custom" mode).
//!
//! The response is a stream of newline-delimited JSON: a conversation frame,
//! the echoed user message, then `streamingVideoGenerationResponse` frames
//! whose `progress` climbs 0 → 100. The final (100%) frame carries the
//! `videoUrl` / `assetId` / thumbnail. Reading the body waits for the whole
//! stream, i.e. until generation finishes.

use crate::client::grok_domain::GrokDomain;
use crate::credentials::grok_cookies::GrokCookies;
use crate::credentials::grok_request_headers::GrokRequestHeaders;
use crate::datatypes::api::file_id::FileId;
use crate::error::categorize_grok_http_error::categorize_grok_http_error;
use crate::error::grok_client_error::GrokClientError;
use crate::error::grok_error::GrokError;
use crate::error::grok_generic_api_error::GrokGenericApiError;
use crate::utils::create_firefox_client::create_firefox_client;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use wreq::header::{ACCEPT, ACCEPT_LANGUAGE, CONTENT_TYPE, COOKIE, ORIGIN, REFERER, USER_AGENT};

use crate::client::browser_user_agents::FIREFOX_143_MAC_USER_AGENT;

const NEW_CONVERSATION_PATH: &str = "/rest/app-chat/conversations/new";

/// The imagine video model.
const VIDEO_MODEL_NAME: &str = "imagine-video-gen";

const CONVERSATION_KIND_IMAGINE: &str = "CONVERSATION_KIND_IMAGINE";

/// Aspect ratios for generated video. Only `3:2` is confirmed from captures;
/// the others mirror the image aspect ratios and are included for parity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoAspectRatio {
  /// 1:1
  Square,
  /// 2:3
  TallTwoByThree,
  /// 3:2
  WideThreeByTwo,
  /// 16:9
  WideSixteenByNine,
  /// 9:16
  TallNineBySixteen,
}

impl VideoAspectRatio {
  pub fn as_grok_str(self) -> &'static str {
    match self {
      Self::Square => "1:1",
      Self::TallTwoByThree => "2:3",
      Self::WideThreeByTwo => "3:2",
      Self::WideSixteenByNine => "16:9",
      Self::TallNineBySixteen => "9:16",
    }
  }
}

/// Output resolution for generated video.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoResolution {
  /// 480p
  Sd480p,
  /// 720p
  Hd720p,
}

impl VideoResolution {
  pub fn as_grok_str(self) -> &'static str {
    match self {
      Self::Sd480p => "480p",
      Self::Hd720p => "720p",
    }
  }
}

pub struct GenerateVideoRequest<'a> {
  /// The input image asset id to animate. It may be a Grok-generated image or
  /// an uploaded (untrusted) image — the wire shape is identical either way.
  pub input_asset_id: &'a str,

  /// Optional text prompt. When present the request uses "custom" mode;
  /// otherwise plain "normal" mode.
  pub prompt: Option<&'a str>,

  pub aspect_ratio: VideoAspectRatio,
  pub resolution: VideoResolution,

  /// Video length in seconds. The web app defaults to 6.
  pub duration_seconds: u32,
}

pub struct GenerateVideoArgs<'a> {
  pub request: GenerateVideoRequest<'a>,
  pub credentials: &'a GrokCookies,
  /// Optional captured statsig/tracing headers (see [`GrokRequestHeaders`]).
  pub request_headers: Option<&'a GrokRequestHeaders>,
  pub domain_override: Option<&'a GrokDomain>,
  pub request_timeout: Option<Duration>,
}

/// The result of a completed generation stream.
#[derive(Clone, Debug, Default)]
pub struct GenerateVideoResponse {
  pub conversation_id: Option<String>,
  /// Available as soon as generation starts.
  pub video_id: Option<FileId>,
  /// Bucket path of the finished video (present once progress reaches 100).
  pub video_url: Option<String>,
  /// Bucket path of the preview image.
  pub thumbnail_url: Option<String>,
  pub asset_id: Option<String>,
  pub width: Option<u32>,
  pub height: Option<u32>,
  /// Highest progress seen in the stream (100 when finished).
  pub progress: Option<u32>,
}

/// Send the request and wait for the streamed generation to finish, returning
/// the completed [`GenerateVideoResponse`].
pub async fn generate_video(args: GenerateVideoArgs<'_>) -> Result<GenerateVideoResponse, GrokError> {
  let domain = args.domain_override.unwrap_or(&GrokDomain::DefaultDomain);
  let client = create_firefox_client()?;

  let mut request_builder = client.post(request_url(domain))
      .header(USER_AGENT, FIREFOX_143_MAC_USER_AGENT)
      .header(ACCEPT, "*/*")
      .header(ACCEPT_LANGUAGE, "en-US,en;q=0.5")
      .header(CONTENT_TYPE, "application/json")
      .header(ORIGIN, "https://grok.com")
      .header(REFERER, "https://grok.com/imagine")
      .header(COOKIE, args.credentials.to_string())
      .header("sec-fetch-dest", "empty")
      .header("sec-fetch-mode", "cors")
      .header("sec-fetch-site", "same-origin");

  if let Some(headers) = args.request_headers {
    request_builder = headers.apply(request_builder);
  }

  if let Some(timeout) = args.request_timeout {
    request_builder = request_builder.timeout(timeout);
  }

  let wire = GenerateVideoWireRequest::from_request(&args.request);

  let http_request = request_builder.json(&wire)
      .build()
      .map_err(|err| {
        error!("Error building generate_video request: {:?}", err);
        GrokClientError::WreqClientError(err)
      })?;

  let response = client.execute(http_request)
      .await
      .map_err(|err| {
        error!("Error sending generate_video request: {:?}", err);
        GrokGenericApiError::WreqError(err)
      })?;

  let status = response.status();
  info!("generate_video enqueue status: {}", status.as_u16());

  // Reading the body waits for the whole stream, i.e. until generation is done.
  let body = response.text()
      .await
      .map_err(|err| {
        error!("Error reading generate_video response body: {:?}", err);
        GrokGenericApiError::WreqError(err)
      })?;

  if !status.is_success() {
    error!("generate_video returned an error (code {}): {:?}", status.as_u16(), body);
    return Err(categorize_grok_http_error(status, Some(&body)));
  }

  Ok(parse_generate_video_stream(&body))
}

fn request_url(domain: &GrokDomain) -> String {
  format!("{}{}", domain.get_domain(), NEW_CONVERSATION_PATH)
}

/// Parse the newline-delimited JSON stream into a [`GenerateVideoResponse`].
/// Unparseable lines (e.g. a truncated final chunk) are skipped.
pub fn parse_generate_video_stream(body: &str) -> GenerateVideoResponse {
  let mut response = GenerateVideoResponse::default();

  for line in body.lines() {
    let line = line.trim();
    if line.is_empty() {
      continue;
    }

    let Ok(frame) = serde_json::from_str::<StreamFrame>(line) else {
      continue;
    };
    let Some(result) = frame.result else {
      continue;
    };

    if let Some(conversation) = result.conversation {
      if conversation.conversation_id.is_some() {
        response.conversation_id = conversation.conversation_id;
      }
    }

    if let Some(video) = result.response.and_then(|r| r.streaming_video) {
      if video.video_id.is_some() {
        response.video_id = video.video_id.map(FileId);
      }
      if video.progress.is_some() {
        response.progress = video.progress;
      }
      // The 100% frame carries the finished-video fields.
      if video.video_url.is_some() {
        response.video_url = video.video_url;
        response.asset_id = video.asset_id;
        response.thumbnail_url = video.thumbnail_url;
        response.width = video.width;
        response.height = video.height;
      }
    }
  }

  response
}

// ----- Wire request -----

/// Field order mirrors the web app's wire payload (observed 2026-08-23).
#[derive(Serialize)]
struct GenerateVideoWireRequest {
  #[serde(rename = "modelName")]
  model_name: &'static str,
  message: String,
  #[serde(rename = "enableImageStreaming")]
  enable_image_streaming: bool,
  #[serde(rename = "enableSideBySide")]
  enable_side_by_side: bool,
  #[serde(rename = "sendFinalMetadata")]
  send_final_metadata: bool,
  #[serde(rename = "responseMetadata")]
  response_metadata: ResponseMetadata,
  #[serde(rename = "mediaGenInput")]
  media_gen_input: MediaGenInput,
  kind: &'static str,
}

#[derive(Serialize)]
struct ResponseMetadata {
  experiments: Vec<serde_json::Value>,
  #[serde(rename = "modelConfigOverride")]
  model_config_override: ModelConfigOverride,
}

#[derive(Serialize)]
struct ModelConfigOverride {
  #[serde(rename = "modelMap")]
  model_map: ModelMap,
}

/// Serializes to an empty object `{}`.
#[derive(Serialize)]
struct ModelMap {}

#[derive(Serialize)]
struct MediaGenInput {
  #[serde(rename = "imageToVideo")]
  image_to_video: ImageToVideo,
}

#[derive(Serialize)]
struct ImageToVideo {
  #[serde(rename = "prompt", skip_serializing_if = "Option::is_none")]
  prompt: Option<String>,
  #[serde(rename = "inputAssets")]
  input_assets: Vec<String>,
  #[serde(rename = "aspectRatio")]
  aspect_ratio: &'static str,
  duration: u32,
  #[serde(rename = "resolutionName")]
  resolution_name: &'static str,
  mode: &'static str,
}

impl GenerateVideoWireRequest {
  fn from_request(request: &GenerateVideoRequest) -> Self {
    // With a prompt the web app sends "<prompt> --mode=custom"; without one it
    // sends just "--mode=normal".
    let (message, mode, prompt) = match request.prompt {
      Some(prompt) => (format!("{prompt} --mode=custom"), "custom", Some(prompt.to_string())),
      None => ("--mode=normal".to_string(), "normal", None),
    };

    Self {
      model_name: VIDEO_MODEL_NAME,
      message,
      enable_image_streaming: true,
      enable_side_by_side: true,
      send_final_metadata: true,
      response_metadata: ResponseMetadata {
        experiments: Vec::new(),
        model_config_override: ModelConfigOverride { model_map: ModelMap {} },
      },
      media_gen_input: MediaGenInput {
        image_to_video: ImageToVideo {
          prompt,
          input_assets: vec![request.input_asset_id.to_string()],
          aspect_ratio: request.aspect_ratio.as_grok_str(),
          duration: request.duration_seconds,
          resolution_name: request.resolution.as_grok_str(),
          mode,
        },
      },
      kind: CONVERSATION_KIND_IMAGINE,
    }
  }
}

// ----- Wire response (streamed frames) -----

#[derive(Deserialize)]
struct StreamFrame {
  result: Option<StreamResult>,
}

#[derive(Deserialize)]
struct StreamResult {
  conversation: Option<StreamConversation>,
  response: Option<StreamResponse>,
}

#[derive(Deserialize)]
struct StreamConversation {
  #[serde(rename = "conversationId")]
  conversation_id: Option<String>,
}

#[derive(Deserialize)]
struct StreamResponse {
  #[serde(rename = "streamingVideoGenerationResponse")]
  streaming_video: Option<StreamingVideo>,
}

#[derive(Deserialize)]
struct StreamingVideo {
  #[serde(rename = "videoId")]
  video_id: Option<String>,
  progress: Option<u32>,
  #[serde(rename = "assetId")]
  asset_id: Option<String>,
  #[serde(rename = "videoUrl")]
  video_url: Option<String>,
  #[serde(rename = "thumbnailImageUrl")]
  thumbnail_url: Option<String>,
  width: Option<u32>,
  height: Option<u32>,
}

#[cfg(test)]
mod tests {
  use super::*;

  // Cargo runs tests with the crate root as the working directory.
  fn load(file_name: &str) -> String {
    std::fs::read_to_string(format!("test_data/endpoint_responses/{file_name}")).unwrap()
  }

  mod wire_format_tests {
    use super::*;

    #[test]
    fn url_uses_domain() {
      assert_eq!(
        request_url(&GrokDomain::DefaultDomain),
        "https://grok.com/rest/app-chat/conversations/new",
      );
      let custom = GrokDomain::Custom("http://localhost:8080".to_string());
      assert_eq!(request_url(&custom), "http://localhost:8080/rest/app-chat/conversations/new");
    }

    #[test]
    fn aspect_ratio_and_resolution_strings() {
      assert_eq!(VideoAspectRatio::Square.as_grok_str(), "1:1");
      assert_eq!(VideoAspectRatio::TallTwoByThree.as_grok_str(), "2:3");
      assert_eq!(VideoAspectRatio::WideThreeByTwo.as_grok_str(), "3:2");
      assert_eq!(VideoAspectRatio::WideSixteenByNine.as_grok_str(), "16:9");
      assert_eq!(VideoAspectRatio::TallNineBySixteen.as_grok_str(), "9:16");
      assert_eq!(VideoResolution::Sd480p.as_grok_str(), "480p");
      assert_eq!(VideoResolution::Hd720p.as_grok_str(), "720p");
    }

    // Real "from generation" send frame (no prompt, normal mode, 480p).
    #[test]
    fn normal_mode_request_matches_capture() {
      let request = GenerateVideoRequest {
        input_asset_id: "11111111-1111-4111-8111-111111111111",
        prompt: None,
        aspect_ratio: VideoAspectRatio::WideThreeByTwo,
        resolution: VideoResolution::Sd480p,
        duration_seconds: 6,
      };
      let ours = serde_json::to_value(GenerateVideoWireRequest::from_request(&request)).unwrap();
      let captured: serde_json::Value =
          serde_json::from_str(&load("generate_video_from_generation_request.json")).unwrap();
      assert_eq!(ours, captured);
    }

    // Real "from upload" send frame (with prompt, custom mode, 720p).
    #[test]
    fn custom_mode_request_matches_capture() {
      let request = GenerateVideoRequest {
        input_asset_id: "22222222-2222-4222-8222-222222222222",
        prompt: Some("Windmill spins as the camera orbits the night scene. Stars above."),
        aspect_ratio: VideoAspectRatio::WideThreeByTwo,
        resolution: VideoResolution::Hd720p,
        duration_seconds: 6,
      };
      let ours = serde_json::to_value(GenerateVideoWireRequest::from_request(&request)).unwrap();
      let captured: serde_json::Value =
          serde_json::from_str(&load("generate_video_from_upload_request.json")).unwrap();
      assert_eq!(ours, captured);
    }
  }

  mod response_parsing_tests {
    use super::*;

    #[test]
    fn parses_real_streamed_response() {
      let response = parse_generate_video_stream(&load("generate_video_response.txt"));

      assert_eq!(
        response.conversation_id.as_deref(),
        Some("8b592ec5-b3ec-4cb4-b1f3-ddfd87b20625"),
      );
      assert_eq!(
        response.video_id.as_ref().map(|id| id.0.as_str()),
        Some("1622a562-971d-479b-b4be-44c96f3cd813"),
      );
      assert_eq!(response.progress, Some(100));
      assert!(response.video_url.as_deref().unwrap().ends_with("generated_video.mp4"));
      assert_eq!(
        response.asset_id.as_deref(),
        Some("1622a562-971d-479b-b4be-44c96f3cd813"),
      );
      assert!(response.thumbnail_url.as_deref().unwrap().ends_with("preview_image.jpg"));
      assert_eq!(response.width, Some(832));
      assert_eq!(response.height, Some(352));
    }

    #[test]
    fn truncated_lines_are_skipped() {
      // A conversation frame plus a garbage/truncated line.
      let body = "{\"result\":{\"conversation\":{\"conversationId\":\"abc\"}}}\n{\"result\":{\"resp";
      let response = parse_generate_video_stream(body);
      assert_eq!(response.conversation_id.as_deref(), Some("abc"));
      assert!(response.video_id.is_none());
    }
  }

  mod real_wire_tests {
    use super::*;
    use crate::credentials::grok_cookies::GrokCookies;
    use crate::endpoint_bindings::generate_image_websocket::grok_generate_image_websocket::GrokImageWebsocket;
    use crate::endpoint_bindings::generate_image_websocket::messages::websocket_client_message::FastAspectRatio;
    use crate::endpoint_bindings::list_assets::list_assets::{ListAssetsArgs, ListAssetsRequest};
    use crate::endpoint_bindings::old_bindings::index_page::signature::generate_xsid::{generate_xsid, GenerateXsidArgs};
    use crate::endpoint_bindings::upload_file::grok_upload_file::{grok_upload_file, GrokUploadFileArgs, GrokUploadFileRequest, PathOrFile};
    use crate::recipes::request_client_secrets::{request_client_secrets, RequestClientSecretsArgs};
    use crate::test_utils::grok_test_secrets::load_grok_test_secrets;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    use errors::AnyhowResult;
    use log::{info, LevelFilter};
    use std::path::Path;

    const TEST_IMAGE_PATH: &str = "test_data/images/test_upload.png";
    const VIDEO_TIMEOUT: Duration = Duration::from_secs(180);

    /// This endpoint validates `x-statsig-id`, so we must mint a fresh one for
    /// this exact path+method (a stale/replayed id — or none — gets a 403
    /// "page is out of date"). Everything needed is scraped from the index page
    /// + xsid script.
    ///
    /// NB: as of 2026-08 the `index_page` scrapers are themselves out of date
    /// (Grok changed index.html), so this currently fails to find the
    /// verification token. These live tests will pass once that machinery is
    /// refreshed; the request/response binding itself is verified by the
    /// offline tests above.
    async fn fresh_statsig_headers(cookies: &GrokCookies) -> AnyhowResult<GrokRequestHeaders> {
      let secrets = request_client_secrets(RequestClientSecretsArgs { cookies }).await?;
      let statsig_id = generate_xsid(GenerateXsidArgs {
        path: NEW_CONVERSATION_PATH,
        method: "POST",
        verification_token: &secrets.verification_token,
        svg_data: &secrets.svg_path_data,
        numbers: &secrets.numbers,
      })?;
      Ok(GrokRequestHeaders { statsig_id: Some(statsig_id), ..Default::default() })
    }

    /// Generate a video from `input_asset_id` and assert we get a finished
    /// video URL back.
    async fn generate_and_assert(input_asset_id: &str, prompt: Option<&str>) -> AnyhowResult<()> {
      let secrets = load_grok_test_secrets()?;
      let headers = fresh_statsig_headers(&secrets.cookies).await?;

      let response = generate_video(GenerateVideoArgs {
        request: GenerateVideoRequest {
          input_asset_id,
          prompt,
          aspect_ratio: VideoAspectRatio::WideThreeByTwo,
          resolution: VideoResolution::Sd480p,
          duration_seconds: 6,
        },
        credentials: &secrets.cookies,
        request_headers: Some(&headers),
        domain_override: None,
        request_timeout: Some(VIDEO_TIMEOUT),
      }).await?;

      info!("generate_video response: {:?}", response);
      assert_eq!(response.progress, Some(100), "video did not finish");
      assert!(response.video_url.is_some(), "expected a finished video url");
      Ok(())
    }

    // Way 1: from a text prompt — generate a seed image from text, then animate
    // it. (We only have image-to-video wire captures, so a text prompt becomes
    // a generated seed image.)
    #[tokio::test]
    #[ignore] // Hits the real website; spends image + video quota.
    async fn generate_video_from_text_prompt() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Info);
      let secrets = load_grok_test_secrets()?;

      let websocket = GrokImageWebsocket::connect(secrets.cookies.as_str()).await?;
      websocket.send_fast_image_prompt_with_retry(
        "a lone lighthouse on a cliff at dusk",
        FastAspectRatio::WideThreeByTwo,
      ).await?;
      let images = websocket.collect_images(Duration::from_secs(60)).await?;
      let seed = images.first().expect("expected a generated seed image");
      // The image url ends with `/generated/{asset_id}/image.jpg`.
      let asset_id = seed.url.split('/').nth_back(1).expect("asset id in url").to_string();

      generate_and_assert(&asset_id, Some("the light beam sweeps across the water")).await
    }

    // Way 2: from an existing generated image (fetched from the asset list).
    #[tokio::test]
    #[ignore] // Hits the real website; spends video quota.
    async fn generate_video_from_generated_image() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Info);
      let secrets = load_grok_test_secrets()?;

      let assets = ListAssetsArgs {
        request: ListAssetsRequest { page_size: Some(20) },
        credentials: &secrets.cookies,
        domain_override: None,
        request_headers: Some(&secrets.headers),
        request_timeout: None,
      }.send().await?;

      let image = assets.assets.iter()
          .find(|a| a.mime_type.as_deref() == Some("image/jpeg"))
          .expect("expected at least one generated image asset");

      generate_and_assert(&image.asset_id, None).await
    }

    // Way 3: from an uploaded (untrusted) image.
    #[tokio::test]
    #[ignore] // Hits the real website; spends video quota.
    async fn generate_video_from_uploaded_image() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Info);
      let secrets = load_grok_test_secrets()?;

      let upload = grok_upload_file(GrokUploadFileArgs {
        request: GrokUploadFileRequest { file: PathOrFile::Path(Path::new(TEST_IMAGE_PATH)) },
        cookie: secrets.cookies.as_str(),
        domain_override: None,
        request_timeout: Some(Duration::from_secs(30)),
      }).await?;
      let asset_id = upload.file_id.expect("upload should yield a file id").0;

      generate_and_assert(&asset_id, Some("slow cinematic zoom")).await
    }
  }
}

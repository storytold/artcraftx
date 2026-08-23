use crate::client::midjourney_hostname::MidjourneyHostname;
use crate::error::midjourney_api_error::MidjourneyApiError;
use crate::error::midjourney_client_error::MidjourneyClientError;
use crate::error::midjourney_error::MidjourneyError;
use cloudflare_errors::filter_cloudflare_errors::filter_cloudflare_errors;
use log::error;
use serde::{Deserialize, Serialize};
use wreq::Client;
use wreq_util::Emulation;

/// Logs a prompt to `/api/prompt-session-log`.
///
/// The web client fires this alongside each imagine submission (see the
/// `08`/`11` captures). It is telemetry, not the job-enqueue call — that is
/// `submit_job`. We mirror it so our traffic matches the browser's.
pub struct PromptSessionLogRequest<'a> {
  pub hostname: MidjourneyHostname,
  pub cookie_header: String,

  /// A client-generated session id (UUID) grouping prompts in one sitting.
  pub session_id: &'a str,

  /// Monotonic index of this prompt within the session, starting at 0.
  pub sequence_index: u32,

  /// Epoch milliseconds when the prompt was captured.
  pub captured_at_ms: u64,

  /// The prompt text WITHOUT the trailing `--` parameters.
  pub raw_text: &'a str,

  /// The model version string, e.g. `"8.2"`.
  pub version: &'a str,
}

#[derive(Debug, Clone)]
pub struct PromptSessionLogResponse {
  /// The `status` field from the response body (e.g. `"success"`).
  pub status: Option<String>,
}

pub async fn prompt_session_log(
  req: PromptSessionLogRequest<'_>,
) -> Result<PromptSessionLogResponse, MidjourneyError> {
  let cookie_header = req.cookie_header.trim();
  if cookie_header.len() < 20 {
    error!("Cookie header is too short (len: {}): {}", cookie_header.len(), cookie_header);
    return Err(MidjourneyClientError::CookieTooShort.into());
  }

  let client = Client::builder()
      .emulation(Emulation::Firefox139)
      .build()
      .map_err(MidjourneyClientError::WreqError)?;

  let referer = format!("https://{}/imagine", req.hostname.as_str());
  let url = format!("https://{}/api/prompt-session-log", req.hostname.as_str());

  #[derive(Serialize)]
  struct RawRequest<'a> {
    session_id: &'a str,
    sequence_index: u32,
    captured_at_ms: u64,
    source: &'a str,
    raw_text: &'a str,
    route: &'a str,
    in_conversation_mode: bool,
    version: &'a str,
    video_mode: bool,
    image_prompt_count: u32,
    style_ref_count: u32,
    character_ref_count: u32,
    depth_ref_count: u32,
    starting_frame_count: u32,
    ending_frame_count: u32,
    openai_item_id: Option<String>,
  }

  let body = RawRequest {
    session_id: req.session_id,
    sequence_index: req.sequence_index,
    captured_at_ms: req.captured_at_ms,
    source: "imagine_text",
    raw_text: req.raw_text,
    route: "/imagine",
    in_conversation_mode: false,
    version: req.version,
    video_mode: false,
    image_prompt_count: 0,
    style_ref_count: 0,
    character_ref_count: 0,
    depth_ref_count: 0,
    starting_frame_count: 0,
    ending_frame_count: 0,
    openai_item_id: None,
  };

  let http_request = client.post(url)
      .header("cookie", cookie_header)
      .header("Referer", &referer)
      .header("Referrer-Policy", "origin-when-cross-origin")
      .header("accept", "*/*")
      .header("accept-language", "en-US,en;q=0.8")
      .header("content-type", "application/json")
      .header("priority", "u=1, i")
      .header("sec-ch-ua-mobile", "?0")
      .header("sec-fetch-dest", "empty")
      .header("sec-fetch-mode", "cors")
      .header("sec-fetch-site", "same-origin")
      .header("x-csrf-protection", "1")
      .json(&body)
      .build()
      .map_err(MidjourneyClientError::WreqError)?;

  let response = client.execute(http_request)
      .await
      .map_err(|e| MidjourneyApiError::NetworkError(e.to_string()))?;

  let status = response.status();
  let response_body = response.text().await
      .map_err(|e| MidjourneyApiError::NetworkError(e.to_string()))?;

  if !status.is_success() {
    if let Err(err) = filter_cloudflare_errors(status.as_u16(), &response_body) {
      return Err(MidjourneyApiError::CloudflareError(err).into());
    }
    return Err(MidjourneyApiError::UnknownHttpFailure {
      status_code: status.as_u16(),
      body: response_body,
    }.into());
  }

  // {"status":"success"}
  #[derive(Deserialize)]
  struct RawResponse {
    status: Option<String>,
  }

  let response = serde_json::from_str::<RawResponse>(&response_body)
      .map_err(MidjourneyApiError::DeserializationError)?;

  Ok(PromptSessionLogResponse {
    status: response.status,
  })
}

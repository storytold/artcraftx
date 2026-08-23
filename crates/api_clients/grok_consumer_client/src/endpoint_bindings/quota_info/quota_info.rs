//! `POST /rest/media/imagine/quota_info` — remaining imagine generation
//! quota per media kind. Captured in
//! `external/requests/sites/grok.com/2026-08-23-imagine/12_quota_info.txt`.

use crate::client::browser_user_agents::FIREFOX_143_MAC_USER_AGENT;
use crate::client::grok_domain::GrokDomain;
use crate::credentials::grok_cookies::GrokCookies;
use crate::error::categorize_grok_http_error::categorize_grok_http_error;
use crate::error::grok_error::GrokError;
use crate::error::grok_generic_api_error::GrokGenericApiError;
use crate::utils::create_firefox_client::create_firefox_client;
use log::error;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use wreq::header::{ACCEPT, ACCEPT_LANGUAGE, CONTENT_TYPE, COOKIE, ORIGIN, REFERER, USER_AGENT};

const QUOTA_INFO_PATH: &str = "/rest/media/imagine/quota_info";

/// Request arguments. The endpoint currently takes none (the body is `{}`);
/// this exists so arguments slot in without changing call sites.
#[derive(Clone, Debug, Default)]
pub struct QuotaInfoRequest {}

pub struct QuotaInfoArgs<'a> {
  pub request: QuotaInfoRequest,
  pub credentials: &'a GrokCookies,
  pub domain_override: Option<&'a GrokDomain>,
  pub request_timeout: Option<Duration>,
}

/// Quota per media kind. Kinds the account has no quota line for are `null`.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaInfoResponse {
  pub image: Option<QuotaInfoBucket>,
  pub image_pro: Option<QuotaInfoBucket>,
  pub image_edit: Option<QuotaInfoBucket>,
  pub video: Option<QuotaInfoBucket>,
  #[serde(rename = "video720p")]
  pub video_720p: Option<QuotaInfoBucket>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaInfoBucket {
  pub available: bool,
  pub remaining_queries: u64,
  /// The rolling quota window, e.g. 86400 (one day).
  pub window_size_seconds: u64,
  /// RFC 3339 timestamp; present when the bucket is exhausted.
  pub next_available_at: Option<String>,
}

/// Wire body — always the empty object.
#[derive(Serialize)]
struct QuotaInfoRawRequest {}

impl QuotaInfoArgs<'_> {
  pub async fn send(&self) -> Result<QuotaInfoResponse, GrokError> {
    let domain = self.domain_override.unwrap_or(&GrokDomain::DefaultDomain);
    let client = create_firefox_client()?;

    let mut request_builder = client.post(request_url(domain))
        .header(USER_AGENT, FIREFOX_143_MAC_USER_AGENT)
        .header(ACCEPT, "*/*")
        .header(ACCEPT_LANGUAGE, "en-US,en;q=0.5")
        .header(CONTENT_TYPE, "application/json")
        .header(ORIGIN, "https://grok.com")
        .header(REFERER, "https://grok.com/imagine/saved")
        .header(COOKIE, self.credentials.to_string())
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-origin");

    if let Some(timeout) = self.request_timeout {
      request_builder = request_builder.timeout(timeout);
    }

    let http_request = request_builder.json(&QuotaInfoRawRequest {})
        .build()
        .map_err(|err| {
          error!("Error building quota_info request: {:?}", err);
          GrokGenericApiError::WreqError(err)
        })?;

    let response = client.execute(http_request)
        .await
        .map_err(|err| {
          error!("Error sending quota_info request: {:?}", err);
          GrokGenericApiError::WreqError(err)
        })?;

    let status = response.status();

    let response_body = response.text()
        .await
        .map_err(|err| {
          error!("Error reading quota_info response body: {:?}", err);
          GrokGenericApiError::WreqError(err)
        })?;

    if !status.is_success() {
      error!("quota_info returned an error (code {}): {:?}", status.as_u16(), response_body);
      return Err(categorize_grok_http_error(status, Some(&response_body)));
    }

    parse_response(&response_body)
  }
}

fn request_url(domain: &GrokDomain) -> String {
  format!("{}{}", domain.get_domain(), QUOTA_INFO_PATH)
}

fn parse_response(body: &str) -> Result<QuotaInfoResponse, GrokError> {
  serde_json::from_str(body)
      .map_err(|err| {
        GrokGenericApiError::SerdeResponseParseErrorWithBody(err, body.to_string()).into()
      })
}

#[cfg(test)]
mod tests {
  use super::*;

  // Real response from 12_quota_info.txt (nothing identifying in this one).
  const CAPTURED_RESPONSE: &str = r#"{"image":null,"imagePro":{"available":true,"remainingQueries":4,"windowSizeSeconds":86400},"imageEdit":null,"video":null,"video720p":{"available":true,"remainingQueries":0,"windowSizeSeconds":86400,"nextAvailableAt":"2026-08-24T21:00:41.230702760Z"}}"#;

  mod wire_format_tests {
    use super::*;

    #[test]
    fn url_uses_the_default_domain() {
      assert_eq!(
        request_url(&GrokDomain::DefaultDomain),
        "https://grok.com/rest/media/imagine/quota_info",
      );
    }

    #[test]
    fn url_respects_a_domain_override() {
      let domain = GrokDomain::Custom("http://localhost:8080".to_string());
      assert_eq!(
        request_url(&domain),
        "http://localhost:8080/rest/media/imagine/quota_info",
      );
    }

    #[test]
    fn body_is_the_empty_object() {
      let body = serde_json::to_string(&QuotaInfoRawRequest {}).unwrap();
      assert_eq!(body, "{}");
    }
  }

  mod response_parsing_tests {
    use super::*;

    #[test]
    fn parses_the_captured_response() {
      let response = parse_response(CAPTURED_RESPONSE).unwrap();

      assert!(response.image.is_none());
      assert!(response.image_edit.is_none());
      assert!(response.video.is_none());

      let image_pro = response.image_pro.unwrap();
      assert!(image_pro.available);
      assert_eq!(image_pro.remaining_queries, 4);
      assert_eq!(image_pro.window_size_seconds, 86400);
      assert!(image_pro.next_available_at.is_none());

      let video_720p = response.video_720p.unwrap();
      assert!(video_720p.available);
      assert_eq!(video_720p.remaining_queries, 0);
      assert_eq!(
        video_720p.next_available_at.as_deref(),
        Some("2026-08-24T21:00:41.230702760Z"),
      );
    }
  }

  mod real_wire_tests {
    use super::*;
    use crate::test_utils::get_test_cookies::get_typed_test_cookies;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    use errors::AnyhowResult;
    use log::LevelFilter;

    #[tokio::test]
    #[ignore] // Hits the real website; requires local test cookies.
    async fn fetch_quota_info() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Info);
      let cookies = get_typed_test_cookies()?;

      let args = QuotaInfoArgs {
        request: QuotaInfoRequest::default(),
        credentials: &cookies,
        domain_override: None,
        request_timeout: None,
      };

      let response = args.send().await?;
      println!("[test] quota_info: {:?}", response);
      Ok(())
    }
  }
}

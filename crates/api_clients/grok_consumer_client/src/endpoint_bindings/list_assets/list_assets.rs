//! `GET /rest/assets` — list the user's generated imagine assets (newest
//! first). Captured in
//! `external/requests/sites/grok.com/2026-08-23-imagine/11_list_assets.txt`.

use crate::client::browser_user_agents::FIREFOX_143_MAC_USER_AGENT;
use crate::client::grok_domain::GrokDomain;
use crate::credentials::grok_cookies::GrokCookies;
use crate::error::categorize_grok_http_error::categorize_grok_http_error;
use crate::error::grok_error::GrokError;
use crate::error::grok_generic_api_error::GrokGenericApiError;
use crate::utils::create_firefox_client::create_firefox_client;
use log::error;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
use wreq::header::{ACCEPT, ACCEPT_LANGUAGE, COOKIE, REFERER, USER_AGENT};

const LIST_ASSETS_PATH: &str = "/rest/assets";

/// Page size used when the request doesn't specify one (the website's own).
const DEFAULT_PAGE_SIZE: u32 = 40;

/// Fixed query parameters observed in the capture.
const ORDER_BY: &str = "ORDER_BY_CREATE_TIME";
const WORKSPACE_KIND: &str = "WORKSPACE_KIND_IMAGINE_ALL";

#[derive(Clone, Debug, Default)]
pub struct ListAssetsRequest {
  /// Number of assets per page. Defaults to 40 (the website's own).
  pub page_size: Option<u32>,
}

pub struct ListAssetsArgs<'a> {
  pub request: ListAssetsRequest,
  pub credentials: &'a GrokCookies,
  pub domain_override: Option<&'a GrokDomain>,
  pub request_timeout: Option<Duration>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ListAssetsResponse {
  pub assets: Vec<GrokAsset>,
}

/// One generated (or uploaded) file. Also appears as `latestAssetMetadata`
/// in the imagine conversation listing.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrokAsset {
  pub asset_id: String,
  /// e.g. "image/jpeg", "video/mp4"
  pub mime_type: Option<String>,
  /// e.g. "image.jpg", "generated_video.mp4"
  pub name: Option<String>,
  pub size_bytes: Option<u64>,
  /// RFC 3339, e.g. "2026-08-23T20:01:10.223Z"
  pub create_time: Option<String>,
  /// Bucket path of the primary file,
  /// e.g. `users/{user_id}/generated/{asset_id}/image.jpg`.
  pub key: Option<String>,
  /// Extra file paths and flags, e.g. "preview-image", "original-image",
  /// "thumbhash", "moderated".
  #[serde(default)]
  pub aux_keys: HashMap<String, String>,
  pub response_id: Option<String>,
  pub is_deleted: Option<bool>,
  pub is_latest: Option<bool>,
  pub source_conversation_id: Option<String>,
  pub width: Option<u32>,
  pub height: Option<u32>,
  /// The generation request that produced the asset. Shape varies by mode
  /// (`textToImage`, `imageToVideo`, ...), so it stays untyped for now.
  pub media_gen_input: Option<serde_json::Value>,
}

impl ListAssetsArgs<'_> {
  pub async fn send(&self) -> Result<ListAssetsResponse, GrokError> {
    let domain = self.domain_override.unwrap_or(&GrokDomain::DefaultDomain);
    let client = create_firefox_client()?;

    let mut request_builder = client.get(request_url(domain, &self.request))
        .header(USER_AGENT, FIREFOX_143_MAC_USER_AGENT)
        .header(ACCEPT, "*/*")
        .header(ACCEPT_LANGUAGE, "en-US,en;q=0.5")
        .header(REFERER, "https://grok.com/imagine/saved")
        .header(COOKIE, self.credentials.to_string())
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-origin");

    if let Some(timeout) = self.request_timeout {
      request_builder = request_builder.timeout(timeout);
    }

    let http_request = request_builder
        .build()
        .map_err(|err| {
          error!("Error building list_assets request: {:?}", err);
          GrokGenericApiError::WreqError(err)
        })?;

    let response = client.execute(http_request)
        .await
        .map_err(|err| {
          error!("Error sending list_assets request: {:?}", err);
          GrokGenericApiError::WreqError(err)
        })?;

    let status = response.status();

    let response_body = response.text()
        .await
        .map_err(|err| {
          error!("Error reading list_assets response body: {:?}", err);
          GrokGenericApiError::WreqError(err)
        })?;

    if !status.is_success() {
      error!("list_assets returned an error (code {}): {:?}", status.as_u16(), response_body);
      return Err(categorize_grok_http_error(status, Some(&response_body)));
    }

    parse_response(&response_body)
  }
}

fn request_url(domain: &GrokDomain, request: &ListAssetsRequest) -> String {
  let page_size = request.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
  format!(
    "{}{}?pageSize={}&orderBy={}&workspaceKind={}",
    domain.get_domain(), LIST_ASSETS_PATH, page_size, ORDER_BY, WORKSPACE_KIND,
  )
}

fn parse_response(body: &str) -> Result<ListAssetsResponse, GrokError> {
  serde_json::from_str(body)
      .map_err(|err| {
        GrokGenericApiError::SerdeResponseParseErrorWithBody(err, body.to_string()).into()
      })
}

#[cfg(test)]
mod tests {
  use super::*;

  // Real response from 11_list_assets.txt, truncated to two assets and
  // scrubbed: user/asset/conversation/response ids replaced with synthetic
  // UUIDs. Structure and remaining values are as captured.
  const SCRUBBED_RESPONSE: &str = r#"{"assets":[{"assetId":"11111111-1111-4111-8111-111111111111","mimeType":"video/mp4","name":"generated_video.mp4","sizeBytes":5800655,"createTime":"2026-08-23T20:01:10.223Z","lastUseTime":"2026-08-23T20:01:10.235Z","summary":"","previewImageKey":"","key":"users/00000000-0000-4000-8000-000000000000/generated/11111111-1111-4111-8111-111111111111/generated_video.mp4","auxKeys":{"image_edit_is_root_user_uploaded":"false","image_references":"[\"https://assets.grok.com/users/00000000-0000-4000-8000-000000000000/generated/33333333-3333-4333-8333-333333333333/image.jpg\"]","is_ext":"false","is_root_celebrity":"false","moderated":"false","preview-image":"users/00000000-0000-4000-8000-000000000000/generated/11111111-1111-4111-8111-111111111111/preview_image.jpg","thumbhash":"DvgJBIAHZ4iOhImJhnn2cY0Ldw=="},"responseId":"44444444-4444-4444-8444-444444444444","isDeleted":false,"fileSource":"IMAGINE_GENERATED_FILE_SOURCE","sourceConversationId":"55555555-5555-4555-8555-555555555555","isModelGenerated":true,"updateTime":"2026-08-23T20:01:10.235Z","isLatest":true,"inlineStatus":"DEFAULT_ARTIFACT_INLINE_STATUS","isRootAssetCreatedByModel":true,"rootAssetSourceConversationId":"55555555-5555-4555-8555-555555555555","sharedWithTeam":false,"sharedWithUserIds":[],"isPublic":false,"width":736,"height":400,"rRated":false,"thumbhash":"DvgJBIAHZ4iOhImJhnn2cY0Ldw==","mediaGenInput":{"imageToVideo":{"inputAssets":["33333333-3333-4333-8333-333333333333"],"aspectRatio":"16:9","duration":6,"resolutionName":"480p","modelName":"imagine-video-gen","mode":"normal","skipAudio":false}},"ownerUserId":"00000000-0000-4000-8000-000000000000"},{"assetId":"22222222-2222-4222-8222-222222222222","mimeType":"image/jpeg","name":"image.jpg","sizeBytes":517902,"createTime":"2026-08-23T19:56:19.599Z","lastUseTime":"2026-08-23T19:56:19.599Z","summary":"","previewImageKey":"","key":"users/00000000-0000-4000-8000-000000000000/generated/22222222-2222-4222-8222-222222222222/image.jpg","auxKeys":{"image_edit_is_root_user_uploaded":"false","is_root_celebrity":"false","moderated":"false","original-image":"users/00000000-0000-4000-8000-000000000000/generated/22222222-2222-4222-8222-222222222222/original_image.jpg","thumbhash":"DvgJBIAYV3iPdIeIdnjSV28HVQ=="},"responseId":"66666666-6666-4666-8666-666666666666","isDeleted":false,"fileSource":"IMAGINE_GENERATED_FILE_SOURCE","sourceConversationId":"77777777-7777-4777-8777-777777777777","isModelGenerated":true,"updateTime":"2026-08-23T19:56:19.599Z","isLatest":true,"inlineStatus":"DEFAULT_ARTIFACT_INLINE_STATUS","isRootAssetCreatedByModel":true,"rootAssetSourceConversationId":"77777777-7777-4777-8777-777777777777","sharedWithTeam":false,"sharedWithUserIds":[],"isPublic":false,"width":1280,"height":720,"rRated":false,"thumbhash":"DvgJBIAYV3iPdIeIdnjSV28HVQ==","mediaGenInput":{"textToImage":{"prompt":"A pirate ship in the middle of the forest","numOfImages":2,"aspectRatio":"16:9","modelName":"imagine-image-gen","mode":"quality"}},"ownerUserId":"00000000-0000-4000-8000-000000000000"}]}"#;

  mod wire_format_tests {
    use super::*;

    #[test]
    fn url_defaults_to_page_size_40() {
      let request = ListAssetsRequest { page_size: None };
      assert_eq!(
        request_url(&GrokDomain::DefaultDomain, &request),
        "https://grok.com/rest/assets?pageSize=40&orderBy=ORDER_BY_CREATE_TIME&workspaceKind=WORKSPACE_KIND_IMAGINE_ALL",
      );
    }

    #[test]
    fn url_respects_page_size_and_domain_overrides() {
      let request = ListAssetsRequest { page_size: Some(7) };
      let domain = GrokDomain::Custom("http://localhost:8080".to_string());
      assert_eq!(
        request_url(&domain, &request),
        "http://localhost:8080/rest/assets?pageSize=7&orderBy=ORDER_BY_CREATE_TIME&workspaceKind=WORKSPACE_KIND_IMAGINE_ALL",
      );
    }
  }

  mod response_parsing_tests {
    use super::*;

    #[test]
    fn parses_the_scrubbed_captured_response() {
      let response = parse_response(SCRUBBED_RESPONSE).unwrap();
      assert_eq!(response.assets.len(), 2);

      let video = &response.assets[0];
      assert_eq!(video.asset_id, "11111111-1111-4111-8111-111111111111");
      assert_eq!(video.mime_type.as_deref(), Some("video/mp4"));
      assert_eq!(video.name.as_deref(), Some("generated_video.mp4"));
      assert_eq!(video.size_bytes, Some(5800655));
      assert_eq!(video.width, Some(736));
      assert_eq!(video.height, Some(400));
      assert_eq!(video.is_deleted, Some(false));
      assert_eq!(
        video.aux_keys.get("preview-image").map(String::as_str),
        Some("users/00000000-0000-4000-8000-000000000000/generated/11111111-1111-4111-8111-111111111111/preview_image.jpg"),
      );
      let video_input = video.media_gen_input.as_ref().unwrap();
      assert_eq!(video_input["imageToVideo"]["duration"], 6);

      let image = &response.assets[1];
      assert_eq!(image.asset_id, "22222222-2222-4222-8222-222222222222");
      assert_eq!(image.mime_type.as_deref(), Some("image/jpeg"));
      assert_eq!(
        image.source_conversation_id.as_deref(),
        Some("77777777-7777-4777-8777-777777777777"),
      );
      let image_input = image.media_gen_input.as_ref().unwrap();
      assert_eq!(
        image_input["textToImage"]["prompt"],
        "A pirate ship in the middle of the forest",
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
    async fn fetch_assets() -> AnyhowResult<()> {
      setup_test_logging(LevelFilter::Info);
      let cookies = get_typed_test_cookies()?;

      let args = ListAssetsArgs {
        request: ListAssetsRequest { page_size: Some(5) },
        credentials: &cookies,
        domain_override: None,
        request_timeout: None,
      };

      let response = args.send().await?;
      for asset in &response.assets {
        println!("[test] asset: {} ({:?})", asset.asset_id, asset.mime_type);
      }
      Ok(())
    }
  }
}

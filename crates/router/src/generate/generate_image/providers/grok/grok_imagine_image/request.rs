use grok_consumer_client::endpoint_bindings::generate_image_websocket::grok_generate_image_websocket::GrokImageWebsocket;
use grok_consumer_client::endpoint_bindings::generate_image_websocket::messages::websocket_client_message::{FastAspectRatio, QualityAspectRatio};
use grok_consumer_client::prompt_flags::PromptFlags;
use std::time::Duration;

use crate::api::router_aspect_ratio::RouterAspectRatio;
use crate::client::router_grok_client::RouterGrokClient;
use crate::errors::artcraft_router_error::ArtcraftRouterError;
use crate::errors::provider_error::ProviderError;
use crate::generate::generate_image::generate_image_response::{
  GenerateImageResponse, GrokImageResponsePayload,
};

/// Grok's imagine websocket delivers finished images synchronously; wait up to
/// this long for them to arrive.
const IMAGE_TIMEOUT: Duration = Duration::from_secs(120);

/// A ready-to-send first-party Grok Imagine image request. Fast vs quality
/// ("pro") is `enable_pro`; the two modes support slightly different aspect
/// ratios (see [`fast_aspect`] / [`quality_aspect`]).
#[derive(Debug, Clone)]
pub struct GrokImagineImageRequestState {
  pub prompt: String,
  pub aspect_ratio: RouterAspectRatio,
  pub enable_pro: bool,
}

impl GrokImagineImageRequestState {
  /// Open the imagine websocket, submit the prompt, and wait for the finished
  /// image URLs. This blocks for the whole generation — call it off a
  /// user-facing thread (e.g. the background fulfillment worker).
  pub async fn send(
    &self,
    client: &RouterGrokClient,
  ) -> Result<GenerateImageResponse, ArtcraftRouterError> {
    let websocket = GrokImageWebsocket::connect(&client.cookie_header)
        .await
        .map_err(|err| grok_err("connect", err))?;

    let flags = PromptFlags::default();

    if self.enable_pro {
      websocket
          .send_quality_image_prompt_with_retry(&self.prompt, quality_aspect(self.aspect_ratio), &flags)
          .await
    } else {
      websocket
          .send_fast_image_prompt_with_retry(&self.prompt, fast_aspect(self.aspect_ratio), &flags)
          .await
    }
    .map_err(|err| grok_err("send", err))?;

    let images = websocket
        .collect_images(IMAGE_TIMEOUT)
        .await
        .map_err(|err| grok_err("collect", err))?;

    let image_urls: Vec<String> = images.into_iter().map(|image| image.url).collect();
    if image_urls.is_empty() {
      return Err(ArtcraftRouterError::Provider(ProviderError::Grok(
        "Grok returned no images".to_string(),
      )));
    }

    Ok(GenerateImageResponse::Grok(GrokImageResponsePayload { image_urls }))
  }
}

fn grok_err(stage: &str, err: impl std::fmt::Display) -> ArtcraftRouterError {
  ArtcraftRouterError::Provider(ProviderError::Grok(format!("{stage}: {err}")))
}

/// Map a router aspect ratio to fast mode's supported set (nearest fit).
fn fast_aspect(ratio: RouterAspectRatio) -> FastAspectRatio {
  match ratio {
    RouterAspectRatio::WideThreeByTwo | RouterAspectRatio::WideFourByThree => FastAspectRatio::WideThreeByTwo,
    RouterAspectRatio::WideSixteenByNine
    | RouterAspectRatio::Wide
    | RouterAspectRatio::WideTwentyOneByNine => FastAspectRatio::WideSixteenByNine,
    RouterAspectRatio::TallTwoByThree
    | RouterAspectRatio::TallThreeByFour
    | RouterAspectRatio::TallFourByFive => FastAspectRatio::TallTwoByThree,
    RouterAspectRatio::TallNineBySixteen
    | RouterAspectRatio::Tall
    | RouterAspectRatio::TallNineByTwentyOne => FastAspectRatio::TallNineBySixteen,
    _ => FastAspectRatio::Square,
  }
}

/// Map a router aspect ratio to quality mode's larger supported set.
fn quality_aspect(ratio: RouterAspectRatio) -> QualityAspectRatio {
  match ratio {
    RouterAspectRatio::WideThreeByTwo => QualityAspectRatio::WideThreeByTwo,
    RouterAspectRatio::WideFourByThree => QualityAspectRatio::WideFourByThree,
    RouterAspectRatio::WideSixteenByNine | RouterAspectRatio::Wide => QualityAspectRatio::WideSixteenByNine,
    RouterAspectRatio::WideTwentyOneByNine => QualityAspectRatio::WideTwentyOneByNine,
    RouterAspectRatio::TallTwoByThree
    | RouterAspectRatio::TallThreeByFour
    | RouterAspectRatio::TallFourByFive => QualityAspectRatio::TallTwoByThree,
    RouterAspectRatio::TallNineBySixteen
    | RouterAspectRatio::Tall
    | RouterAspectRatio::TallNineByTwentyOne => QualityAspectRatio::TallNineBySixteen,
    _ => QualityAspectRatio::Square,
  }
}

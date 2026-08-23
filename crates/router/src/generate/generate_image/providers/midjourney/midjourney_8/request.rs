use midjourney_client::recipes::channel_id::ChannelId;
use midjourney_client::recipes::text_to_image::{text_to_image, TextToImageArgs, TextToImageRequest};

use crate::client::router_midjourney_client::RouterMidjourneyClient;
use crate::errors::artcraft_router_error::ArtcraftRouterError;
use crate::errors::provider_error::ProviderError;
use crate::generate::generate_image::generate_image_response::{
  GenerateImageResponse, MidjourneyImageResponsePayload,
};

/// A ready-to-send first-party Midjourney v8 request. The `prompt` is already
/// fully composed, including trailing `--` parameters (aspect ratio, `--v 8.2`).
#[derive(Debug, Clone)]
pub struct MidjourneyMidjourney8RequestState {
  pub prompt: String,
}

impl MidjourneyMidjourney8RequestState {
  pub async fn send(
    &self,
    client: &RouterMidjourneyClient,
  ) -> Result<GenerateImageResponse, ArtcraftRouterError> {
    let channel_id = ChannelId::UserId(client.user_id.clone());

    let response = text_to_image(TextToImageArgs {
      request: TextToImageRequest {
        prompt: &self.prompt,
        channel_id: &channel_id,
      },
      cookie_header: &client.cookie_header,
      hostname: None,
      browser: Some(client.browser.clone()),
    })
    .await
    .map_err(|err| ArtcraftRouterError::Provider(ProviderError::Midjourney(err)))?;

    let job_id = response.maybe_job_id.ok_or_else(|| {
      // Surface Midjourney's own human-readable message(s) (e.g.
      // "subscription_required", banned prompt), not a debug dump.
      let detail = response
          .maybe_errors
          .map(|errors| {
            errors
                .into_iter()
                .filter_map(|error| error.message)
                .collect::<Vec<String>>()
                .join("; ")
          })
          .filter(|message| !message.is_empty())
          .unwrap_or_else(|| "Midjourney rejected the request with no detail".to_string());
      ArtcraftRouterError::Provider(ProviderError::MidjourneySubmitRejected(detail))
    })?;

    Ok(GenerateImageResponse::Midjourney(MidjourneyImageResponsePayload { job_id }))
  }
}

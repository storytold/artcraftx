use log::info;

use crate::api::image_list_ref::ImageListRef;
use crate::errors::artcraft_router_error::ArtcraftRouterError;
use crate::generate::generate_image::image_generation_draft_context::ImageGenerationDraftContext;
use crate::generate::generate_image::providers::higgsfield::image_request::HiggsfieldImageRequest;
use crate::generate::generate_image::providers::higgsfield::request::HiggsfieldImageRequestState;
use crate::utils::higgsfield_media::{upload_media_list, HiggsfieldMediaKind};

/// A planned Higgsfield image request whose reference images still have to
/// be uploaded. `to_request` does the uploads and produces the sendable state.
#[derive(Clone, Debug)]
pub struct HiggsfieldImageDraftState {
  /// Everything but the references, already snapped to Higgsfield's menus.
  pub request: HiggsfieldImageRequest,
  pub unhandled_request_state: Option<HiggsfieldImageRemainingItems>,
}

#[derive(Clone, Debug)]
pub struct HiggsfieldImageRemainingItems {
  pub reference_images: Option<ImageListRef>,
}

impl HiggsfieldImageDraftState {
  pub fn reference_image_count(&self) -> usize {
    self.unhandled_request_state.as_ref()
        .and_then(|remaining| remaining.reference_images.as_ref())
        .map_or(0, ImageListRef::len)
  }

  pub async fn to_request(
    &mut self,
    draft_context: &ImageGenerationDraftContext<'_>,
  ) -> Result<HiggsfieldImageRequestState, ArtcraftRouterError> {
    let client = draft_context.get_higgsfield_client_ref()?;
    let session = &client.session;

    let mut reference_images = Vec::new();
    if let Some(remaining) = self.unhandled_request_state.take() {
      let map = draft_context.media_file_to_artcraft_url_map;
      // The image generator's references skip the IP check (as the web app
      // does); only the Seedance video models insist on it.
      reference_images = upload_media_list(
        session,
        remaining.reference_images.map(Into::into),
        HiggsfieldMediaKind::Image,
        false,
        map,
      ).await?;
      info!("Uploaded {} reference image(s) to Higgsfield for {}", reference_images.len(), self.request.model_label());
    }

    Ok(HiggsfieldImageRequestState {
      request: self.request.clone().with_reference_images(reference_images),
    })
  }
}

use artcraft_client::utils::api_host::ApiHost;
use crate::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::events::functional_events::text_to_image_generation_complete_event::{GeneratedImage, TextToImageGenerationCompleteEvent};
use anyhow::anyhow;
use sqlite_identifiers::task_type::TaskType;
use sqlite_identifiers::tauri_command_caller::TauriCommandCaller;
use errors::AnyhowResult;
use log::error;
use sqlite_database::queries::task::Task;
use artcraft_client::credentials::storyteller_credential_set::StorytellerCredentialSet;
use artcraft_client::endpoints::media_files::list_batch_generated_redux_media_files::list_batch_generated_redux_media_files;
use tauri::AppHandle;
use sqlite_identifiers::batch_generation_token::BatchGenerationToken;

pub async fn maybe_handle_text_to_image_complete_event(
  app: &AppHandle,
  maybe_creds: Option<&StorytellerCredentialSet>,
  task: &Task,
  batch_token: &BatchGenerationToken,
) -> AnyhowResult<()> {

  match task.task_type {
    TaskType::ImageGeneration => {} // NB: Fall-through
    _ => return Ok(()),
  }

  match task.frontend_caller {
    Some(TauriCommandCaller::TextToImage) => {} // NB: Fall-through
    _ => return Ok(()),
  }

  let event = handle_batch(
    app,
    maybe_creds,
    task,
    batch_token,
  ).await?;

  if let Err(err) = event.send(&app) {
    error!("Failed to send TextToImageGenerationCompleteEvent: {:?}", err); // Fail open
  }

  Ok(())
}

async fn handle_batch(
  _app: &AppHandle,
  maybe_creds: Option<&StorytellerCredentialSet>,
  task: &Task,
  batch_token: &BatchGenerationToken,
) -> AnyhowResult<TextToImageGenerationCompleteEvent> {

  let result = list_batch_generated_redux_media_files(
    &ApiHost::Storyteller,
    maybe_creds,
    batch_token,
  ).await?;

  if result.media_files.is_empty() {
    return Err(anyhow!("No media files found for batch token: {}", batch_token));
  }

  let media_files = result.media_files
      .into_iter()
      .map(|file| GeneratedImage {
        media_token: file.token,
        cdn_url: file.media_links.cdn_url,
        maybe_thumbnail_template: file.media_links.maybe_thumbnail_template,
      })
      .collect();

  Ok(TextToImageGenerationCompleteEvent {
    generated_images: media_files,
    maybe_frontend_subscriber_id: task.frontend_subscriber_id.clone(),
    maybe_frontend_subscriber_payload: task.frontend_subscriber_payload.clone(),
  })
}

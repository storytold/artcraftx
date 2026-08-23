use artcraft_client::api_defs::prompts::create_prompt::CreatePromptRequest;
use artcraft_client::api_defs::utils::media_links_to_thumbnail_template::media_links_to_thumbnail_template;
use artcraft_client::credentials::storyteller_credential_set::StorytellerCredentialSet;
use artcraft_client::endpoints::media_files::get_media_file::get_media_file;
use artcraft_client::endpoints::media_files::upload_image_media_file_from_file::{
  upload_image_media_file_from_file, UploadImageFromFileArgs,
};
use artcraft_client::endpoints::prompts::create_prompt::create_prompt;
use artcraft_client::enums::common::generation::common_model_type::CommonModelType;
use artcraft_client::error::api_error::ApiError;
use artcraft_client::error::storyteller_error::StorytellerError;
use artcraft_client::utils::api_host::ApiHost;
use core_types::enums::generation_source::GenerationSource;
use errors::AnyhowResult;
use log::{error, info};
use midjourney_client::utils::image_downloader_client::ImageDownloaderClient;
use sqlite_database::queries::task::Task;
use sqlite_database::queries::update::update_successful_task_status_with_metadata::{
  update_successful_task_status_with_metadata, UpdateSuccessfulTaskArgs,
};
use sqlite_identifiers::enums::task_media_file_class::TaskMediaFileClass;
use sqlite_identifiers::ids::batch_generation_token::BatchGenerationToken;
use tauri::AppHandle;
use uuid_utils::uuid::generate_random_uuid;

use crate::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::events::generation_events::common::{GenerationAction, GenerationServiceProvider};
use crate::events::generation_events::generation_complete_event::GenerationCompleteEvent;
use crate::services::midjourney::threads::events::maybe_handle_text_to_image_complete_event::maybe_handle_text_to_image_complete_event;
use crate::services::midjourney::utils::download_midjourney_image::download_midjourney_image;
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::database::task_database::TaskDatabase;

/// How many images a Midjourney job produces (the 2x2 grid).
const MIDJOURNEY_GRID_SIZE: u8 = 4;

/// The shared completion routine for a Midjourney job: download the four
/// full-resolution images (from the deterministic CDN URL), upload them to
/// Storyteller, mark the task complete, and notify the frontend.
///
/// Both the websocket fast path and the long-polling fallback call this. It is
/// safe to call from both concurrently: it finalizes via
/// `update_successful_task_status_with_metadata`, which no-ops (returns false)
/// once a task is already complete — so whichever path gets there first wins.
///
/// Returns `Ok(true)` if this call finalized the task, `Ok(false)` if it was
/// already finalized by someone else.
pub struct FinalizeMidjourneyJobArgs<'a> {
  pub app_handle: &'a AppHandle,
  pub app_data_root: &'a AppDataRoot,
  pub task_database: &'a TaskDatabase,
  pub storyteller_creds: &'a StorytellerCredentialSet,
  pub image_downloader: &'a ImageDownloaderClient,
  pub midjourney_job_id: &'a str,
  pub local_task: &'a Task,
  pub model_type: CommonModelType,
  pub maybe_full_command: Option<String>,
}

pub async fn finalize_midjourney_job(args: FinalizeMidjourneyJobArgs<'_>) -> AnyhowResult<bool> {
  let FinalizeMidjourneyJobArgs {
    app_handle,
    app_data_root,
    task_database,
    storyteller_creds,
    image_downloader,
    midjourney_job_id,
    local_task,
    model_type,
    maybe_full_command,
  } = args;

  let create_request = CreatePromptRequest {
    uuid_idempotency_token: generate_random_uuid(),
    positive_prompt: maybe_full_command,
    negative_prompt: None,
    model_type: Some(model_type),
    generation_provider: Some(GenerationSource::Midjourney),
    maybe_generation_mode: None,
    maybe_aspect_ratio: None,
    maybe_resolution: None,
    maybe_batch_count: None,
    maybe_generate_audio: None,
    maybe_duration_seconds: None,
  };

  let prompt_response = create_prompt(&ApiHost::Storyteller, Some(storyteller_creds), create_request).await?;
  info!("Created prompt: {:?}", &prompt_response.prompt_token);

  // TODO: Move this from clientside to the backend.
  //  The first upload should produce a batch token that we can reuse.
  let batch_token = BatchGenerationToken::generate();

  let mut maybe_primary_media_file_token = None;

  for index in 0..MIDJOURNEY_GRID_SIZE {
    let download_path = download_midjourney_image(
      image_downloader,
      midjourney_job_id,
      index,
      app_data_root,
    ).await?;

    let mut wait_delay = 0;
    loop {
      let result = upload_image_media_file_from_file(UploadImageFromFileArgs {
        api_host: &ApiHost::Storyteller,
        maybe_creds: Some(storyteller_creds),
        path: &download_path,
        is_intermediate_system_file: false,
        maybe_prompt_token: Some(&prompt_response.prompt_token),
        maybe_batch_token: Some(&batch_token),
        maybe_generation_provider: Some(GenerationSource::Midjourney),
      }).await;

      match result {
        Ok(result) => {
          if maybe_primary_media_file_token.is_none() {
            maybe_primary_media_file_token = Some(result.media_file_token);
          }
          break;
        }
        Err(StorytellerError::Api(ApiError::TooManyRequests(_))) => {
          wait_delay += 10;
          if wait_delay > 60 {
            wait_delay = 60;
          }
          error!("Too many requests uploading Midjourney image; retrying in {}s", wait_delay);
          tokio::time::sleep(std::time::Duration::from_secs(wait_delay)).await;
          continue;
        }
        Err(err) => {
          error!("Failed to upload Midjourney image to backend: {:?}", err);
          return Err(err.into());
        }
      }
    }

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
  }

  let mut maybe_cdn_url = None;
  let mut maybe_thumbnail_url_template = None;

  if let Some(media_file_token) = maybe_primary_media_file_token.as_ref() {
    match get_media_file(&ApiHost::Storyteller, media_file_token).await {
      Ok(response) => {
        maybe_cdn_url = Some(response.media_file.media_links.cdn_url.to_string());
        maybe_thumbnail_url_template =
          media_links_to_thumbnail_template(&response.media_file.media_links).map(|s| s.to_string());
      }
      Err(err) => {
        error!("Failed to look up media file after upload: {:?} (failing open)", err);
      }
    }
  }

  let updated = update_successful_task_status_with_metadata(UpdateSuccessfulTaskArgs {
    db: task_database.get_connection(),
    task_id: &local_task.id,
    maybe_batch_token: Some(&batch_token),
    maybe_primary_media_file_token: maybe_primary_media_file_token.as_ref(),
    maybe_primary_media_file_class: Some(TaskMediaFileClass::Image),
    maybe_primary_media_file_thumbnail_url_template: maybe_thumbnail_url_template.as_deref(),
    maybe_primary_media_file_cdn_url: maybe_cdn_url.as_deref(),
  }).await?;

  if !updated {
    // Another path already finalized this task; don't emit duplicate events.
    return Ok(false);
  }

  let event = GenerationCompleteEvent {
    action: Some(GenerationAction::GenerateImage),
    service: GenerationServiceProvider::Midjourney,
    model: None,
  };
  if let Err(err) = event.send(app_handle) {
    error!("Failed to send GenerationCompleteEvent: {:?}", err); // Fail open
  }

  if let Err(err) = maybe_handle_text_to_image_complete_event(
    app_handle,
    Some(storyteller_creds),
    local_task,
    &batch_token,
  ).await {
    error!("Failed to send text-to-image complete event: {:?}", err);
  }

  Ok(true)
}

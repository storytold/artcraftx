use crate::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::events::generation_events::common::{GenerationAction, GenerationServiceProvider};
use crate::events::generation_events::generation_complete_event::GenerationCompleteEvent;
use crate::error::artcraftx_error::ArtcraftXError;
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;
use crate::state::app_preferences::app_preferences_manager::AppPreferencesManager;
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::database::task_database::TaskDatabase;
use crate::state::downloads::preferred_download_filename::DownloadFilenameParts;
use crate::threads::third_party_task_polling_thread::events::notify_frontend_of_completion::{
  notify_frontend_of_completion, CompletionData,
};
use crate::utils::download::download_url_to_download_dir_via_temp::download_url_to_download_dir_via_temp;
use crate::utils::download::download_url_to_temp_dir::download_url_to_temp_dir;
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
use chrono::Local;
use core_types::enums::generation_source::GenerationSource;
use errors::AnyhowResult;
use grok_consumer_client::endpoint_bindings::generate_image_websocket::grok_generate_image_websocket::CompletedImage;
use log::{error, info, warn};
use reqwest::Url;
use sqlite_database::queries::task::Task;
use sqlite_database::queries::update::update_successful_task_status_with_metadata::{
  update_successful_task_status_with_metadata, UpdateSuccessfulTaskArgs,
};
use sqlite_database::queries::update::update_task_status::{update_task_status, UpdateTaskArgs};
use sqlite_identifiers::enums::task_media_file_class::TaskMediaFileClass;
use sqlite_identifiers::enums::task_status::TaskStatus;
use sqlite_identifiers::ids::batch_generation_token::BatchGenerationToken;
use sqlite_identifiers::ids::media_file_token::MediaFileToken;
use std::time::Duration;
use tauri::AppHandle;
use uuid_utils::uuid::generate_random_uuid;

/// Download filename slug for Grok Imagine images.
const GROK_IMAGE_MODEL_SLUG: &str = "grok_imagine";

const MAX_UPLOAD_RETRIES: u32 = 5;
const INITIAL_RETRY_DELAY_SECS: u64 = 10;

/// A Grok image task finished: save the images to the user's download
/// directory, upload them to ArtCraft when logged in, mark the task complete,
/// and notify the frontend.
pub async fn handle_grok_image_complete(
  app_handle: &AppHandle,
  app_data_root: &AppDataRoot,
  app_preferences: &AppPreferencesManager,
  task_database: &TaskDatabase,
  storyteller_creds_manager: &StorytellerCredentialManager,
  task: &Task,
  images: &[CompletedImage],
) {
  info!("[GrokComplete] Handling completed task {} ({} image(s))", task.id.as_str(), images.len());

  let result = handle_grok_image_complete_inner(
    app_handle,
    app_data_root,
    app_preferences,
    task_database,
    storyteller_creds_manager,
    task,
    images,
  ).await;

  if let Err(err) = result {
    error!("[GrokComplete] Failed to handle task {}: {:?}", task.id.as_str(), err);
  }
}

async fn handle_grok_image_complete_inner(
  app_handle: &AppHandle,
  app_data_root: &AppDataRoot,
  app_preferences: &AppPreferencesManager,
  task_database: &TaskDatabase,
  storyteller_creds_manager: &StorytellerCredentialManager,
  task: &Task,
  images: &[CompletedImage],
) -> AnyhowResult<()> {
  // 1) Always keep a local copy in the user's download directory (fail open).
  save_to_download_directory(app_data_root, app_preferences, task, images).await;

  // 2) Upload to ArtCraft when there's a session; otherwise the task still
  // completes locally with the downloads.
  let maybe_creds = storyteller_creds_manager.get_credentials()?;

  let Some(creds) = maybe_creds else {
    warn!("[GrokComplete] No ArtCraft session; completing task {} without uploading", task.id.as_str());
    update_task_status(UpdateTaskArgs {
      db: task_database.get_connection(),
      task_id: &task.id,
      status: TaskStatus::CompleteSuccess,
    }).await?;

    GenerationCompleteEvent {
      action: Some(GenerationAction::GenerateImage),
      service: GenerationServiceProvider::Grok,
      model: None,
    }.send_infallible(app_handle);
    return Ok(());
  };

  let prompt_token = {
    let prompt = images.iter()
        .map(|image| image.user_prompt.trim())
        .find(|prompt| !prompt.is_empty())
        .map(|prompt| prompt.to_string());

    let response = create_prompt(&ApiHost::Storyteller, Some(&creds), CreatePromptRequest {
      uuid_idempotency_token: generate_random_uuid(),
      positive_prompt: prompt,
      negative_prompt: None,
      model_type: Some(CommonModelType::GrokImagineImage),
      generation_provider: Some(GenerationSource::Grok),
      maybe_generation_mode: None,
      maybe_aspect_ratio: None,
      maybe_resolution: None,
      maybe_batch_count: None,
      maybe_generate_audio: None,
      maybe_duration_seconds: None,
    }).await?;
    info!("[GrokComplete] Created prompt {:?} for task {}", response.prompt_token, task.id.as_str());
    response.prompt_token
  };

  // TODO: Move this from clientside to the backend.
  //  The first upload should produce a batch token that we can reuse.
  let batch_token = BatchGenerationToken::generate();
  let mut maybe_primary_media_file_token: Option<MediaFileToken> = None;

  for (index, image) in images.iter().enumerate() {
    info!("[GrokComplete] Uploading image {} of {} for task {} ...", index + 1, images.len(), task.id.as_str());
    let file = download_url_to_temp_dir(&image.url, app_data_root).await?;

    let media_token = upload_with_retry(&creds, file.path(), &prompt_token, &batch_token).await?;
    if maybe_primary_media_file_token.is_none() {
      maybe_primary_media_file_token = Some(media_token);
    }
  }

  let mut maybe_cdn_url: Option<Url> = None;
  let mut maybe_thumbnail_url_template = None;

  if let Some(media_file_token) = maybe_primary_media_file_token.as_ref() {
    match get_media_file(&ApiHost::Storyteller, media_file_token).await {
      Ok(response) => {
        maybe_cdn_url = Some(response.media_file.media_links.cdn_url.clone());
        maybe_thumbnail_url_template = media_links_to_thumbnail_template(&response.media_file.media_links)
            .map(|s| s.to_string());
      }
      Err(err) => {
        error!("[GrokComplete] Failed to look up media file after upload: {:?} (failing open)", err);
      }
    }
  }

  let maybe_cdn_url_str = maybe_cdn_url.as_ref().map(|url| url.to_string());

  let updated = update_successful_task_status_with_metadata(UpdateSuccessfulTaskArgs {
    db: task_database.get_connection(),
    task_id: &task.id,
    maybe_batch_token: Some(&batch_token),
    maybe_primary_media_file_token: maybe_primary_media_file_token.as_ref(),
    maybe_primary_media_file_class: Some(TaskMediaFileClass::Image),
    maybe_primary_media_file_cdn_url: maybe_cdn_url_str.as_deref(),
    maybe_primary_media_file_thumbnail_url_template: maybe_thumbnail_url_template.as_deref(),
  }).await?;

  if updated {
    if let Some(primary_token) = maybe_primary_media_file_token {
      let completion = CompletionData {
        primary_media_file_token: primary_token,
        maybe_cdn_url,
        maybe_thumbnail_url_template,
        maybe_batch_token: Some(batch_token),
        media_class: TaskMediaFileClass::Image,
      };
      notify_frontend_of_completion(app_handle, &ApiHost::Storyteller, Some(&creds), task, &completion).await;
    }
  }

  info!("[GrokComplete] Task {} fully handled", task.id.as_str());
  Ok(())
}

// ── Helpers ──

/// Save every image into the user's configured download directory (temp dir
/// first, then moved into place), named per their filename convention. Fails
/// open per file.
async fn save_to_download_directory(
  app_data_root: &AppDataRoot,
  app_preferences: &AppPreferencesManager,
  task: &Task,
  images: &[CompletedImage],
) {
  let app_prefs = match app_preferences.get() {
    Ok(prefs) => prefs,
    Err(err) => {
      error!("[GrokComplete] Can't read app preferences; skipping downloads for task {}: {:?}", task.id.as_str(), err);
      return;
    }
  };

  let download_time = Local::now();

  for (index, image) in images.iter().enumerate() {
    let url = match Url::parse(&image.url) {
      Ok(url) => url,
      Err(err) => {
        error!("[GrokComplete] Bad image URL for task {}: {} ({:?})", task.id.as_str(), image.url, err);
        continue;
      }
    };

    let extension = url.path().rsplit('.').next().unwrap_or("jpg").to_string();

    let filename = app_prefs.downloads.preferred_download_filename.build_filename(&DownloadFilenameParts {
      model_slug: GROK_IMAGE_MODEL_SLUG,
      download_time,
      maybe_batch_index: (images.len() > 1).then(|| index + 1),
      extension: &extension,
    });

    match download_url_to_download_dir_via_temp(&url, Some(&filename), app_data_root, &app_prefs).await {
      Ok(path) => info!("[GrokComplete] Saved task {} image to {:?}", task.id.as_str(), path),
      Err(ArtcraftXError::CannotDownloadFilePathAlreadyExists { path }) => {
        info!("[GrokComplete] Task {} image already saved: {:?}", task.id.as_str(), path);
      }
      Err(err) => error!("[GrokComplete] Failed to save task {} image {}: {:?}", task.id.as_str(), image.url, err),
    }
  }
}

async fn upload_with_retry(
  creds: &StorytellerCredentialSet,
  path: &std::path::Path,
  prompt_token: &sqlite_identifiers::ids::prompt_token::PromptToken,
  batch_token: &BatchGenerationToken,
) -> AnyhowResult<MediaFileToken> {
  let mut retry_delay_secs = INITIAL_RETRY_DELAY_SECS;

  for attempt in 1..=MAX_UPLOAD_RETRIES {
    let result = upload_image_media_file_from_file(UploadImageFromFileArgs {
      api_host: &ApiHost::Storyteller,
      maybe_creds: Some(creds),
      path,
      is_intermediate_system_file: false,
      maybe_prompt_token: Some(prompt_token),
      maybe_batch_token: Some(batch_token),
      maybe_generation_provider: Some(GenerationSource::Grok),
    }).await;

    match result {
      Ok(uploaded) => return Ok(uploaded.media_file_token),
      Err(StorytellerError::Api(ApiError::TooManyRequests(_))) if attempt < MAX_UPLOAD_RETRIES => {
        warn!(
          "[GrokComplete] Upload rate-limited (429), retrying in {}s (attempt {}/{})",
          retry_delay_secs, attempt, MAX_UPLOAD_RETRIES,
        );
        tokio::time::sleep(Duration::from_secs(retry_delay_secs)).await;
        retry_delay_secs = (retry_delay_secs * 2).min(60);
      }
      Err(err) => return Err(err.into()),
    }
  }

  unreachable!("loop returns on the final attempt")
}

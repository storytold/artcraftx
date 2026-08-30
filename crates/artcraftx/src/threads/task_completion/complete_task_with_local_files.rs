use crate::state::app_preferences::app_preferences_manager::AppPreferencesManager;
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::database::task_database::TaskDatabase;
use crate::threads::task_completion::save_results_to_download_dir::save_results_to_download_dir;
use crate::threads::task_completion::upload_results_to_artcraft::{upload_results_to_artcraft, CompletionPrompt};
use crate::threads::third_party_task_polling_thread::events::notify_frontend_of_completion::{
  notify_frontend_of_completion, notify_generation_complete, CompletionData,
};
use crate::utils::download::record_task_download_locations::record_task_download_locations;
use anyhow::bail;
use artcraft_client::credentials::storyteller_credential_set::StorytellerCredentialSet;
use artcraft_client::utils::api_host::ApiHost;
use core_types::enums::generation_source::GenerationSource;
use errors::AnyhowResult;
use log::{info, warn};
use sqlite_database::queries::task::Task;
use sqlite_database::queries::update::update_successful_task_status_with_metadata::{
  update_successful_task_status_with_metadata, UpdateSuccessfulTaskArgs,
};
use sqlite_identifiers::enums::task_media_file_class::TaskMediaFileClass;
use std::path::PathBuf;
use tauri::AppHandle;

pub struct CompleteTaskArgs<'a> {
  pub app_handle: &'a AppHandle,
  pub app_data_root: &'a AppDataRoot,
  pub app_preferences: &'a AppPreferencesManager,
  pub task_database: &'a TaskDatabase,

  /// ArtCraft session to upload the results to. Without one the task still
  /// completes locally with the saved downloads.
  pub maybe_storyteller_creds: Option<&'a StorytellerCredentialSet>,

  pub task: &'a Task,

  /// Provider recorded on the uploaded media files.
  pub generation_provider: GenerationSource,

  /// What kind of files these are (picks the upload endpoint and is stored on
  /// the task).
  pub media_class: TaskMediaFileClass,

  /// How to associate the uploads with a Storyteller prompt record.
  pub prompt: CompletionPrompt,

  /// Used in download filenames when the task has no model type recorded.
  pub fallback_model_slug: &'a str,

  /// The result files, already fetched to local disk (normally the temp
  /// directory), in generation order. The first is the primary file.
  pub local_files: &'a [PathBuf],
}

/// Deliver a finished generation:
///
/// 1. save every file to the user's download directory (fail open per file),
/// 2. upload to ArtCraft when logged in,
/// 3. mark the task complete (a no-op if another path already did — the
///    Midjourney websocket and long-poller can race),
/// 4. record where the files landed on the task,
/// 5. notify the frontend.
///
/// Returns `Ok(true)` if this call completed the task, `Ok(false)` if it was
/// already complete. Errors leave the task pending so the poller retries.
pub async fn complete_task_with_local_files(args: CompleteTaskArgs<'_>) -> AnyhowResult<bool> {
  let CompleteTaskArgs {
    app_handle,
    app_data_root,
    app_preferences,
    task_database,
    maybe_storyteller_creds,
    task,
    generation_provider,
    media_class,
    prompt,
    fallback_model_slug,
    local_files,
  } = args;

  let task_id = task.id.as_str();

  if local_files.is_empty() {
    bail!("Task {} completed with no result files", task_id);
  }

  info!("[TaskCompletion] Delivering {} file(s) for task {} ({:?})", local_files.len(), task_id, generation_provider);

  let downloaded = save_results_to_download_dir(
    app_data_root,
    app_preferences,
    task,
    fallback_model_slug,
    local_files,
  );

  let maybe_uploaded = match maybe_storyteller_creds {
    Some(creds) => {
      let uploaded = upload_results_to_artcraft(
        creds,
        task,
        generation_provider,
        media_class,
        prompt,
        local_files,
      ).await?;
      Some(uploaded)
    }
    None => {
      warn!("[TaskCompletion] No ArtCraft session; completing task {} without uploading", task_id);
      None
    }
  };

  let maybe_cdn_url_str = maybe_uploaded.as_ref()
      .and_then(|uploaded| uploaded.maybe_cdn_url.as_ref())
      .map(|url| url.to_string());

  let updated = update_successful_task_status_with_metadata(UpdateSuccessfulTaskArgs {
    db: task_database.get_connection(),
    task_id: &task.id,
    maybe_batch_token: maybe_uploaded.as_ref().and_then(|uploaded| uploaded.maybe_batch_token.as_ref()),
    maybe_primary_media_file_token: maybe_uploaded.as_ref().map(|uploaded| &uploaded.primary_media_file_token),
    maybe_primary_media_file_class: Some(media_class),
    maybe_primary_media_file_cdn_url: maybe_cdn_url_str.as_deref(),
    maybe_primary_media_file_thumbnail_url_template: maybe_uploaded.as_ref()
        .and_then(|uploaded| uploaded.maybe_thumbnail_url_template.as_deref()),
  }).await?;

  if !updated {
    info!("[TaskCompletion] Task {} was already completed elsewhere; not notifying again", task_id);
    return Ok(false);
  }

  record_task_download_locations(task_database, &task.id, &downloaded).await;

  match maybe_uploaded {
    Some(uploaded) => {
      let completion = CompletionData {
        primary_media_file_token: uploaded.primary_media_file_token,
        maybe_cdn_url: uploaded.maybe_cdn_url,
        maybe_thumbnail_url_template: uploaded.maybe_thumbnail_url_template,
        maybe_batch_token: uploaded.maybe_batch_token,
        media_class,
      };
      notify_frontend_of_completion(app_handle, &ApiHost::Storyteller, maybe_storyteller_creds, task, &completion).await;
    }
    None => notify_generation_complete(app_handle, task),
  }

  info!("[TaskCompletion] Task {} fully handled", task_id);
  Ok(true)
}

use crate::services::sora::threads::sora_task_polling::helpers::download_extension::DownloadExtension;
use crate::services::sora::threads::sora_task_polling::helpers::generation_type::GenerationType;
use crate::state::app_preferences::app_preferences_manager::AppPreferencesManager;
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::data_dir::subdirectory::trait_data_subdir::DataSubdir;
use crate::state::database::task_database::TaskDatabase;
use crate::threads::task_completion::complete_task_with_local_files::{complete_task_with_local_files, CompleteTaskArgs};
use crate::threads::task_completion::upload_results_to_artcraft::CompletionPrompt;
use artcraft_client::credentials::storyteller_credential_set::StorytellerCredentialSet;
use artcraft_client::enums::common::generation::common_model_type::CommonModelType;
use core_types::enums::generation_source::GenerationSource;
use errors::AnyhowResult;
use log::{error, info, warn};
use openai_sora_client::requests::common::task_id::TaskId;
use sqlite_database::queries::task::Task;
use sqlite_identifiers::enums::task_media_file_class::TaskMediaFileClass;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tauri::AppHandle;
use url_utils::download_extension::extract_download_extension_from_url::extract_download_extension_from_url_str;

/// Download filename slug when the task has no model type recorded.
const SORA_FALLBACK_MODEL_SLUG: &str = "sora";

pub struct SuccessfulGeneration {
  pub prompt: Option<String>,
  pub items: Vec<GenerationItem>,
  pub model_type: CommonModelType,
}

pub struct GenerationItem {
  pub item_id: String,
  pub url: String,
}

/// For every succeeded Sora generation we have a local task for: fetch its
/// files, then hand them to the shared completion routine.
pub async fn handle_classic_successful_generations(
  app_handle: &AppHandle,
  app_data_root: &AppDataRoot,
  app_preferences: &AppPreferencesManager,
  task_database: &TaskDatabase,
  maybe_storyteller_creds: Option<&StorytellerCredentialSet>,
  succeeded_tasks_by_id: &HashMap<TaskId, SuccessfulGeneration>,
  sqlite_database_by_sora_task_id: &HashMap<String, Task>,
  recommended_download_extension: DownloadExtension,
) -> AnyhowResult<()> {

  for (task_id, generation) in succeeded_tasks_by_id.iter() {
    let Some(local_task) = sqlite_database_by_sora_task_id.get(task_id.as_str()) else {
      continue; // Task is irrelevant - previously completed, generated elsewhere, etc.
    };

    info!("Task succeeded: {:?}", task_id);

    let generation_type = match generation.model_type {
      CommonModelType::GptImage1 => GenerationType::Image,
      CommonModelType::Sora2 => GenerationType::Video,
      _ => {
        // Fallback
        warn!("Unexpected model type: {:?}", generation.model_type);
        GenerationType::Image
      },
    };

    let media_class = match generation_type {
      GenerationType::Image => TaskMediaFileClass::Image,
      GenerationType::Video => TaskMediaFileClass::Video,
    };

    let mut local_files = Vec::with_capacity(generation.items.len());
    for item in generation.items.iter() {
      info!("Downloading generated file...");
      local_files.push(download_generation_item(item, app_data_root, recommended_download_extension).await?);
    }

    let result = complete_task_with_local_files(CompleteTaskArgs {
      app_handle,
      app_data_root,
      app_preferences,
      task_database,
      maybe_storyteller_creds,
      task: local_task,
      generation_provider: GenerationSource::Sora,
      media_class,
      prompt: CompletionPrompt::Create {
        model_type: generation.model_type,
        maybe_prompt: generation.prompt.clone(),
      },
      fallback_model_slug: SORA_FALLBACK_MODEL_SLUG,
      local_files: &local_files,
    }).await;

    if let Err(err) = result {
      error!("Failed to complete Sora task {}: {:?}", local_task.id.as_str(), err);
    }
  }

  Ok(())
}

async fn download_generation_item(
  generation: &GenerationItem,
  app_data_root: &AppDataRoot,
  recommended_download_extension: DownloadExtension
) -> AnyhowResult<PathBuf> {
  info!("Downloading generation item from URL: {}", generation.url.as_str());

  let response = reqwest::get(&generation.url).await?;
  let image_bytes = response.bytes().await?;

  let extension = extract_download_extension_from_url_str(&generation.url)
      .map(|ext| ext.as_extension_without_period())
      .unwrap_or_else(|| recommended_download_extension.as_extension_without_period());

  let tempdir = app_data_root.temp_dir().path();
  let download_filename = format!("{}.{}", generation.item_id, extension);
  let download_path = tempdir.join(download_filename);

  info!("Writing to path: {:?}", download_path);

  let mut file = File::create(&download_path)?;
  file.write_all(&image_bytes)?;

  Ok(download_path)
}

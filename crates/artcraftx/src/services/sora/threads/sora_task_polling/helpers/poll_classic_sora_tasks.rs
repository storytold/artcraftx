use crate::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::events::generation_events::common::{GenerationAction, GenerationServiceProvider};
use crate::events::generation_events::generation_complete_event::GenerationCompleteEvent;
use crate::events::generation_events::generation_failed_event::GenerationFailedEvent;
use crate::events::sendable_event_trait::SendableEvent;
use crate::state::app_env_configs::app_env_configs::AppEnvConfigs;
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::data_dir::subdirectory::trait_data_subdir::DataSubdir;
use crate::state::task_database::TaskDatabase;
use crate::utils::task_database_pending_statuses::TASK_DATABASE_PENDING_STATUSES;
use crate::services::sora::state::sora_credential_manager::SoraCredentialManager;
use crate::services::sora::state::sora_task_queue::SoraTaskQueue;
use crate::services::sora::threads::sora_task_polling::helpers::download_extension::DownloadExtension;
use crate::services::sora::threads::sora_task_polling::helpers::handle_failed_generations::{handle_classic_failed_generations, FailedGeneration};
use crate::services::sora::threads::sora_task_polling::helpers::handle_successful_generations::{handle_classic_successful_generations, GenerationItem, SuccessfulGeneration};
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;
use artcraft_client::api_defs::prompts::create_prompt::CreatePromptRequest;
use enums::common::generation::common_model_type::CommonModelType;
use errors::AnyhowResult;
use uuid_utils::uuid::generate_random_uuid;
use log::{error, info, warn};
use once_cell::sync::Lazy;
use openai_sora_client::creds::sora_credential_set::SoraCredentialSet;
use openai_sora_client::recipes::list_classic_sora_tasks_with_session_auto_renew::list_classic_sora_tasks_with_session_auto_renew;
use openai_sora_client::requests::common::task_id::TaskId;
use openai_sora_client::requests::list_classic_tasks::list_classic_tasks::TaskStatus;
use reqwest::Url;
use sqlite_database::queries::list_tasks_by_provider_and_status::{list_tasks_by_provider_and_status, ListTasksByProviderAndStatusArgs, TaskList};
use sqlite_database::queries::task::Task;
use sqlite_database::queries::update_task_status::{update_task_status, UpdateTaskArgs};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use artcraft_client::endpoints::media_files::upload_image_media_file_from_file::{upload_image_media_file_from_file, UploadImageFromFileArgs};
use artcraft_client::endpoints::prompts::create_prompt::create_prompt;
use tauri::AppHandle;
use tempdir::TempDir;

pub async fn poll_classic_sora_tasks(
  app_handle: &AppHandle,
  app_env_configs: &AppEnvConfigs,
  task_database: &TaskDatabase,
  sora_creds_manager: &SoraCredentialManager,
  sora_creds: &SoraCredentialSet,
  storyteller_creds_manager: &StorytellerCredentialManager,
  sora_task_queue: &SoraTaskQueue,
  app_data_root: &AppDataRoot,
  local_sqlite_database_by_sora_task_id: &HashMap<String, Task>,
) -> AnyhowResult<()> {

  let (sora_response, maybe_new_creds) =
      list_classic_sora_tasks_with_session_auto_renew(&sora_creds).await?;

  if let Some(new_creds) = maybe_new_creds {
    info!("Saving new credentials.");
    sora_creds_manager.set_credentials(&new_creds)?;
  }

  let sora_items = sora_response.task_responses;

  let mut sora_succeeded_tasks_by_id = HashMap::new();
  let mut sora_failed_tasks_by_id = HashMap::new();

  let storyteller_creds = storyteller_creds_manager.get_credentials_required()?;

  for task in sora_items.iter() {

    match &task.status {
      TaskStatus::Succeeded => {
        sora_succeeded_tasks_by_id.insert(
          task.id.clone(), 
          SuccessfulGeneration {
            prompt: task.prompt.clone(),
            model_type: CommonModelType::GptImage1,
            items: task.generations.iter()
                .map(|gen| {
                  GenerationItem {
                    item_id: gen.id.clone(),
                    url: gen.url.clone(),
                  }
                })
                .collect(),
          });
      }
      TaskStatus::Failed => {
        sora_failed_tasks_by_id.insert(
          task.id.clone(), 
          FailedGeneration {
            reason: None, // TODO: Add reason if available.
          }
        );
      }
      TaskStatus::Queued => {}
      TaskStatus::Running => {}
      TaskStatus::Unknown(unknown_status) => {
        warn!("Unknown task status: {:?}", unknown_status);
      }
    }
  }

  // Clear dead tasks.
  handle_classic_failed_generations(
    &app_handle,
    &task_database,
    &local_sqlite_database_by_sora_task_id,
    &sora_failed_tasks_by_id,
    &sora_task_queue,
  ).await?;

  // Process succeeded tasks.
  handle_classic_successful_generations(
    &app_handle,
    &app_data_root,
    &app_env_configs,
    &task_database,
    &storyteller_creds,
    &sora_succeeded_tasks_by_id,
    &local_sqlite_database_by_sora_task_id,
    DownloadExtension::Png,
  ).await?;

  Ok(())
}

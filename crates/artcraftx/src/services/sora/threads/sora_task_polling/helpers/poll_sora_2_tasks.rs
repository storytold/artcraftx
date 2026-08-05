use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::database::task_database::TaskDatabase;
use crate::services::sora::state::sora_credential_manager::SoraCredentialManager;
use crate::services::sora::state::sora_task_queue::SoraTaskQueue;
use crate::services::sora::threads::sora_task_polling::helpers::download_extension::DownloadExtension;
use crate::services::sora::threads::sora_task_polling::helpers::handle_failed_generations::{handle_classic_failed_generations, FailedGeneration};
use crate::services::sora::threads::sora_task_polling::helpers::handle_successful_generations::{handle_classic_successful_generations, GenerationItem, SuccessfulGeneration};
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;
use artcraft_client::enums::common::generation::common_model_type::CommonModelType;
use errors::AnyhowResult;
use log::info;
use openai_sora_client::creds::sora_credential_set::SoraCredentialSet;
use openai_sora_client::recipes::list_sora2_drafts::list_sora2_drafts_with_session_auto_renew::list_sora2_drafts_with_session_auto_renew;
use openai_sora_client::requests::list_sora2_drafts::list_sora2_drafts::Draft;
use sqlite_database::queries::task::Task;
use std::collections::HashMap;
use tauri::AppHandle;

pub async fn poll_sora_2_tasks(
  app_handle: &AppHandle,
  task_database: &TaskDatabase,
  sora_creds_manager: &SoraCredentialManager,
  sora_creds: &SoraCredentialSet,
  storyteller_creds_manager: &StorytellerCredentialManager,
  sora_task_queue: &SoraTaskQueue,
  app_data_root: &AppDataRoot,
  local_sqlite_database_by_sora_task_id: &HashMap<String, Task>,
) -> AnyhowResult<()> {

  let (sora_response, maybe_new_creds) =
      list_sora2_drafts_with_session_auto_renew(&sora_creds).await?;

  if let Some(new_creds) = maybe_new_creds {
    info!("Saving new credentials.");
    sora_creds_manager.set_credentials(&new_creds)?;
  }

  let drafts = sora_response.drafts;

  let mut sora_succeeded_drafts_by_id = HashMap::new();
  let mut sora_failed_drafts_by_id = HashMap::new();

  let storyteller_creds = storyteller_creds_manager.get_credentials_required()?;

  for draft in drafts.into_iter() {
    match draft {
      Draft::Success(success_draft) => {
        sora_succeeded_drafts_by_id.insert(
          success_draft.task_id,
          SuccessfulGeneration {
            prompt: Some(success_draft.prompt),
            model_type: CommonModelType::Sora2,
            items: vec![GenerationItem {
              item_id: success_draft.id,
              url: success_draft.url,
            }]
          });
      }
      Draft::Failure(failed_draft) => {
        sora_failed_drafts_by_id.insert(
          failed_draft.task_id,
          FailedGeneration {
            reason: Some(failed_draft.reason_message)
          }
        );
      }
    }
  }

  // Clear dead tasks.
  handle_classic_failed_generations(
    &app_handle,
    &task_database,
    &local_sqlite_database_by_sora_task_id,
    &sora_failed_drafts_by_id,
    &sora_task_queue,
  ).await?;

  // Process succeeded tasks.
  handle_classic_successful_generations(
    &app_handle,
    &app_data_root,
    &task_database,
    &storyteller_creds,
    &sora_succeeded_drafts_by_id,
    &local_sqlite_database_by_sora_task_id,
    DownloadExtension::Mp4,
  ).await?;

  Ok(())
}


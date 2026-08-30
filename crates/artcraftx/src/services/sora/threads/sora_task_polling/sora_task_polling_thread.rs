use crate::state::app_preferences::app_preferences_manager::AppPreferencesManager;
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::database::task_database::TaskDatabase;
use crate::database::task_database_pending_statuses::TASK_DATABASE_PENDING_STATUSES;
use crate::services::sora::state::sora_credential_manager::SoraCredentialManager;
use crate::services::sora::state::sora_task_queue::SoraTaskQueue;
use crate::services::sora::threads::sora_task_polling::helpers::poll_classic_sora_tasks::poll_classic_sora_tasks;
use crate::services::sora::threads::sora_task_polling::helpers::poll_sora_2_tasks::poll_sora_2_tasks;
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;
use core_types::enums::generation_source::GenerationSource;
use errors::AnyhowResult;
use log::error;
use sqlite_database::queries::read::list_tasks_by_provider_and_status::{list_tasks_by_provider_and_status, ListTasksByProviderAndStatusArgs};
use sqlite_database::queries::task::Task;
use std::collections::HashMap;
use tauri::AppHandle;

pub async fn sora_task_polling_thread(
  app_handle: AppHandle,
  app_data_root: AppDataRoot,
  task_database: TaskDatabase,
  sora_creds_manager: SoraCredentialManager,
  storyteller_creds_manager: StorytellerCredentialManager,
  sora_task_queue: SoraTaskQueue,
  app_preferences: AppPreferencesManager,
) -> ! {
  loop {
    let res = local_task_polling_loop(
      &app_handle,
      &task_database,
      &sora_creds_manager,
      &storyteller_creds_manager,
      &sora_task_queue,
      &app_data_root,
      &app_preferences,
    ).await;
    if let Err(err) = res {
      error!("An error occurred: {:?}", err);
    }
    tokio::time::sleep(std::time::Duration::from_millis(30_000)).await;
  }
}

async fn local_task_polling_loop(
  app_handle: &AppHandle,
  task_database: &TaskDatabase,
  sora_creds_manager: &SoraCredentialManager,
  storyteller_creds_manager: &StorytellerCredentialManager,
  sora_task_queue: &SoraTaskQueue,
  app_data_root: &AppDataRoot,
  app_preferences: &AppPreferencesManager,
) -> AnyhowResult<()> {
  loop {
    let local_sqlite_database = list_tasks_by_provider_and_status(ListTasksByProviderAndStatusArgs {
      db: task_database.get_connection(),
      provider: GenerationSource::Sora,
      task_statuses: &TASK_DATABASE_PENDING_STATUSES,
    }).await?;

    if local_sqlite_database.tasks.is_empty() {
      // No need to poll if we don't have pending tasks.
      tokio::time::sleep(std::time::Duration::from_millis(2_000)).await;
      continue;
    }

    let creds = sora_creds_manager.get_credentials_required()?;

    // Map of Sora Task ID to Local Task.
    let local_sqlite_database_by_sora_task_id = local_sqlite_database.tasks.iter()
        .filter_map(|task| {
          if let Some(provider_job_id) = &task.provider_job_id {
            Some((provider_job_id.clone(), task.clone()))
          } else {
            None
          }
        })
        .collect::<HashMap<String, Task>>();

    // TODO: Only poll if we have classic items
    
    poll_classic_sora_tasks(
      &app_handle,
      &task_database,
      &sora_creds_manager,
      &creds,
      &storyteller_creds_manager,
      &sora_task_queue,
      &app_data_root,
      &app_preferences,
      &local_sqlite_database_by_sora_task_id,
    ).await?;

    // TODO: Only poll if we have new items
    
    poll_sora_2_tasks(
      &app_handle,
      &task_database,
      &sora_creds_manager,
      &creds,
      &storyteller_creds_manager,
      &sora_task_queue,
      &app_data_root,
      &app_preferences,
      &local_sqlite_database_by_sora_task_id,
    ).await?;

    tokio::time::sleep(std::time::Duration::from_millis(2_000)).await;
  }
}


use crate::state::app_preferences::app_preferences_manager::AppPreferencesManager;
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::database::task_database::TaskDatabase;
use crate::services::sora::state::sora_credential_manager::SoraCredentialManager;
use crate::services::sora::state::sora_task_queue::SoraTaskQueue;
use crate::services::sora::threads::sora_task_polling::sora_task_polling_thread::sora_task_polling_thread;
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;
use errors::AnyhowResult;
use tauri::AppHandle;

pub fn spawn_sora_task_polling_thread(
  app: &AppHandle,
  root: &AppDataRoot,
  task_database: &TaskDatabase,
  sora_credential_manager: &SoraCredentialManager,
  storyteller_creds_manager: &StorytellerCredentialManager,
  sora_task_queue: &SoraTaskQueue,
  app_preferences: &AppPreferencesManager,
) -> AnyhowResult<()> {

  tauri::async_runtime::spawn(sora_task_polling_thread(
    app.clone(),
    root.clone(),
    task_database.clone(),
    sora_credential_manager.clone(),
    storyteller_creds_manager.clone(),
    sora_task_queue.clone(),
    app_preferences.clone(),
  ));

  Ok(())
}

use crate::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::events::generation_events::common::{GenerationAction, GenerationServiceProvider};
use crate::events::generation_events::generation_failed_event::GenerationFailedEvent;
use crate::state::task_database::TaskDatabase;
use crate::services::sora::state::sora_task_queue::SoraTaskQueue;
use sqlite_identifiers::enums::task_status;
use errors::AnyhowResult;
use log::info;
use openai_sora_client::requests::common::task_id::TaskId;
use sqlite_database::queries::task::Task;
use sqlite_database::queries::update_task_status::{update_task_status, UpdateTaskArgs};
use std::collections::HashMap;
use tauri::AppHandle;

pub struct FailedGeneration {
  pub reason: Option<String>,
}

pub async fn handle_classic_failed_generations(
  app_handle: &AppHandle,
  task_database: &TaskDatabase,
  local_sqlite_database_by_sora_task_id: &HashMap<String, Task>,
  sora_failed_tasks_by_id: &HashMap<TaskId, FailedGeneration>,
  sora_task_queue: &SoraTaskQueue,
) -> AnyhowResult<()> {

  for (task_id, task) in sora_failed_tasks_by_id.iter() {
    // Emit events for failed tasks.
    if local_sqlite_database_by_sora_task_id.contains_key(task_id.as_str()) {
      let event = GenerationFailedEvent {
        action: GenerationAction::GenerateImage,
        service: GenerationServiceProvider::Sora,
        model: None,
        reason: task.reason.clone(),
      };

      event.send_infallible(&app_handle);
    }

    // Clear from SQLite task database.
    if let Some(local_task) = local_sqlite_database_by_sora_task_id.get(task_id.as_str()) {
      info!("Marking local task as failed: {:?}", local_task.id);

      let _updated = update_task_status(UpdateTaskArgs {
        db: task_database.get_connection(),
        task_id: &local_task.id,
        status: task_status::TaskStatus::CompleteFailure,
      }).await?;
    }
  }

  // Clear from in memory DB
  // TODO: Remove the in-memory queue in favor of SQLite only.
  let failed_task_ids: Vec<&TaskId> = sora_failed_tasks_by_id.keys().collect();
  sora_task_queue.remove_list(&failed_task_ids)?;

  Ok(())
}

use crate::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::events::generation_events::common::{GenerationAction, GenerationServiceProvider};
use crate::events::generation_events::generation_failed_event::GenerationFailedEvent;
use crate::state::database::task_database::TaskDatabase;
use log::{error, info};
use sqlite_database::queries::task::Task;
use sqlite_database::queries::update::update_task_status::{update_task_status, UpdateTaskArgs};
use sqlite_identifiers::enums::task_status::TaskStatus;
use sqlite_identifiers::enums::task_type::TaskType;
use tauri::AppHandle;

pub async fn handle_higgsfield_failure(
  app_handle: &AppHandle,
  task_database: &TaskDatabase,
  task: &Task,
  reason: &str,
) {
  info!("[HiggsfieldPolling] Marking task {} as failed: {}", task.id.as_str(), reason);

  let update_result = update_task_status(UpdateTaskArgs {
    db: task_database.get_connection(),
    task_id: &task.id,
    status: TaskStatus::CompleteFailure,
  }).await;

  if let Err(err) = update_result {
    error!("[HiggsfieldPolling] Failed to update task status for {}: {:?}", task.id.as_str(), err);
  }

  let event = GenerationFailedEvent {
    action: task_type_to_generation_action(task.task_type),
    service: GenerationServiceProvider::Higgsfield,
    model: None,
    reason: Some(reason.to_string()),
  };

  event.send_infallible(app_handle);
}

fn task_type_to_generation_action(task_type: TaskType) -> GenerationAction {
  match task_type {
    TaskType::ImageGeneration => GenerationAction::GenerateImage,
    TaskType::VideoGeneration => GenerationAction::GenerateVideo,
    TaskType::AudioGeneration => GenerationAction::GenerateAudio,
    TaskType::MeshGeneration => GenerationAction::ImageTo3d,
    TaskType::SplatGeneration => GenerationAction::GenerateGaussian,
  }
}

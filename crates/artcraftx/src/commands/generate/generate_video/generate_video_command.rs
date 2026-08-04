use crate::commands::generate::common::notify_frontend_of_errors::notify_frontend_of_errors;
use crate::commands::generate::generate_error::{BadInputReason, GenerateError, MissingCredentialsReason};
use crate::commands::generate::task_enqueue_success::TaskEnqueueSuccess;
use crate::commands::generate::generate_video::handle_credential_router::handle_credential_router;
use crate::commands::generate::generate_video::request::{
  TauriGenerateVideoErrorType, TauriGenerateVideoRequest, TauriGenerateVideoResponse,
};
use crate::commands::utils::response::failure_response_wrapper::{CommandErrorResponseWrapper, CommandErrorStatus};
use crate::commands::utils::response::shorthand::Response;
use crate::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::events::functional_events::credits_balance_changed_event::CreditsBalanceChangedEvent;
use crate::events::generation_events::generation_enqueue_success_event::GenerationEnqueueSuccessEvent;
use crate::state::artcraft_usage_tracker::artcraft_usage_tracker::ArtcraftUsageTracker;
use crate::state::artcraft_usage_tracker::artcraft_usage_type::{ArtcraftUsagePage, ArtcraftUsageType};
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::task_database::TaskDatabase;
use log::{error, info, warn};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn generate_video_command(
  mut request: TauriGenerateVideoRequest,
  app: AppHandle,
  app_data_root: State<'_, AppDataRoot>,
  artcraft_usage_tracker: State<'_, ArtcraftUsageTracker>,
  task_database: State<'_, TaskDatabase>,
) -> Response<TauriGenerateVideoResponse, TauriGenerateVideoErrorType, ()> {

  info!("generate_video_command called, request: {:?}", request);

  if request.image_media_token.is_none()
      && request.start_frame_image_media_token.is_some() {
    // NB: This is to fix the legacy handlers
    request.image_media_token = request.start_frame_image_media_token.clone();
  }

  if request.image_media_token.is_some()
      && request.start_frame_image_media_token.is_none() {
    // NB: This is to fix the modern handlers
    request.start_frame_image_media_token = request.image_media_token.clone();
  }

  let result = handle_request(
    request,
    &app_data_root,
    &artcraft_usage_tracker,
    &task_database,
  ).await;

  match result {
    Err(err) => {
      error!("error: {:?}", err);

      notify_frontend_of_errors(&app, &err).await;

      let mut status = CommandErrorStatus::ServerError;
      let mut error_type = TauriGenerateVideoErrorType::ServerError;
      let mut error_message = "A server error occurred. Please try again. If it continues, please tell our staff about the problem.".to_string();

      match err {
        GenerateError::BadInput(BadInputReason::NoModelSpecified) => {
          status = CommandErrorStatus::BadRequest;
          error_type = TauriGenerateVideoErrorType::ModelNotSpecified;
          error_message = "No model specified for video generation".to_string();
        }
        GenerateError::NoProviderAvailable => {
          status = CommandErrorStatus::ServerError;
          error_type = TauriGenerateVideoErrorType::NoProviderAvailable;
          error_message = "No configured provider available for video generation".to_string();
        }
        GenerateError::CredentialProblem(_) => {
          status = CommandErrorStatus::Unauthorized;
          error_type = TauriGenerateVideoErrorType::CredentialProblem;
          error_message = "There's a problem with the selected account.".to_string();
        }
        GenerateError::NotYetImplemented(message) => {
          error_message = message;
        }
        GenerateError::MissingCredentials(MissingCredentialsReason::NeedsFalApiKey) => {
          status = CommandErrorStatus::Unauthorized;
          error_type = TauriGenerateVideoErrorType::NeedsFalApiKey;
          error_message = "You need to set a FAL api key".to_string();
        },
        GenerateError::MissingCredentials(MissingCredentialsReason::NeedsStorytellerCredentials) => {
          status = CommandErrorStatus::Unauthorized;
          error_type = TauriGenerateVideoErrorType::NeedsStorytellerCredentials;
          error_message = "You need to be logged into Artcraft.".to_string();
        }
        _ => {}, // Fall-through
      }

      Err(CommandErrorResponseWrapper {
        status,
        error_message: Some(error_message),
        error_type: Some(error_type),
        error_details: None,
      })
    }
    Ok(event) => {
      let event = GenerationEnqueueSuccessEvent {
        action: event.to_frontend_event_action(),
        service: event.to_frontend_event_service(),
        model: event.model,
      };

      if let Err(err) = event.send(&app) {
        error!("Failed to emit event: {:?}", err);
      }

      CreditsBalanceChangedEvent{}.send_infallible(&app);

      Ok(TauriGenerateVideoResponse {}.into())
    }
  }
}

async fn handle_request(
  request: TauriGenerateVideoRequest,
  app_data_root: &AppDataRoot,
  artcraft_usage_tracker: &ArtcraftUsageTracker,
  task_database: &TaskDatabase,
) -> Result<TaskEnqueueSuccess, GenerateError> {

  // Generation is credential-driven: the request names a stored credential,
  // and the router dispatches to that credential's service.
  let result = handle_credential_router(&request, app_data_root).await;

  let success_event = match result {
    Err(err) => return Err(err),
    Ok(event) => event,
  };

  let result = success_event
    .insert_into_task_database_with_frontend_payload(
      task_database,
      request.frontend_caller,
      request.frontend_subscriber_id.as_deref(),
      request.frontend_subscriber_payload.as_deref()
    )
    .await;

  if let Err(err) = result {
    error!("Failed to create task in database: {:?}", err);
  }

  if let Err(err) = artcraft_usage_tracker.record_video_generation(1, ArtcraftUsageType::ImageToResult, ArtcraftUsagePage::VideoPage) {
    warn!("Failed to report usage: {:?}", err);
  }

  Ok(success_event)
}

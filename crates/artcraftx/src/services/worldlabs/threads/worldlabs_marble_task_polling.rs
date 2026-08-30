use crate::state::app_preferences::app_preferences_manager::AppPreferencesManager;
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::data_dir::subdirectory::trait_data_subdir::DataSubdir;
use crate::state::database::task_database::TaskDatabase;
use crate::database::task_database_pending_statuses::TASK_DATABASE_PENDING_STATUSES;
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;
use crate::services::worldlabs::state::worldlabs_credential_manager::WorldlabsCredentialManager;
use core_types::enums::generation_source::GenerationSource;
use sqlite_identifiers::enums::task_media_file_class::TaskMediaFileClass;
use errors::AnyhowResult;
use uuid_utils::uuid::generate_random_uuid;
use log::{error, info};
use sqlite_database::queries::read::list_tasks_by_provider_and_status::{list_tasks_by_provider_and_status, ListTasksByProviderAndStatusArgs, TaskList};
use sqlite_database::queries::task::Task;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use artcraft_client::credentials::storyteller_credential_set::StorytellerCredentialSet;
use crate::threads::task_completion::complete_task_with_local_files::{complete_task_with_local_files, CompleteTaskArgs};
use crate::threads::task_completion::upload_results_to_artcraft::CompletionPrompt;
use tauri::AppHandle;
use worldlabs_consumer_client::api::api_types::world_id::WorldObjectId;
use worldlabs_consumer_client::api::requests::worlds::poll_world_status::{poll_world_status, PollWorldStatusArgs};
use worldlabs_consumer_client::credentials::world_labs_bearer_token::WorldLabsBearerToken;
use worldlabs_consumer_client::credentials::world_labs_cookies::WorldLabsCookies;
use worldlabs_consumer_client::credentials::worldlabs_refresh_token::WorldLabsRefreshToken;

/// Download filename slug when the task has no model type recorded.
const MARBLE_FALLBACK_MODEL_SLUG: &str = "marble";

pub async fn worldlabs_marble_task_polling(
  app_handle: AppHandle,
  app_data_root: AppDataRoot,
  app_preferences: AppPreferencesManager,
  task_database: TaskDatabase,
  creds: WorldlabsCredentialManager,
  storyteller_creds_manager: StorytellerCredentialManager,
) -> ! {
  loop {
    let res = polling_loop(
      &app_handle,
      &app_data_root,
      &app_preferences,
      &task_database,
      &creds,
      &storyteller_creds_manager,
    ).await;
    if let Err(err) = res {
      error!("An error occurred: {:?}", err);
    }
    // NB: Only sleep if an error occurs.
    tokio::time::sleep(Duration::from_millis(30_000)).await;
  }
}

async fn polling_loop(
  app_handle: &AppHandle,
  app_data_root: &AppDataRoot,
  app_preferences: &AppPreferencesManager,
  task_database: &TaskDatabase,
  worldlabs_creds: &WorldlabsCredentialManager,
  storyteller_creds_manager: &StorytellerCredentialManager,
) -> AnyhowResult<()> {
  loop {
    if !worldlabs_creds.do_task_polling()? {
      tokio::time::sleep(Duration::from_millis(10_000)).await;
      continue;
    }

    // Optional: without an ArtCraft session, results are still saved locally.
    let maybe_storyteller_creds = storyteller_creds_manager.get_credentials()?;

    let world_labs_cookies = match worldlabs_creds.maybe_copy_typed_cookies()? {
      Some(cookies) => cookies,
      None => {
        info!("No full WorldLabs cookies");
        tokio::time::sleep(Duration::from_millis(30_000)).await;
        continue;
      }
    };

    let world_labs_bearer = match worldlabs_creds.maybe_copy_bearer_token()? {
      Some(bearer) => bearer,
      None => {
        info!("No full WorldLabs bearer");
        tokio::time::sleep(Duration::from_millis(30_000)).await;
        continue;
      }
    };

    let world_labs_refresh = match worldlabs_creds.maybe_copy_refresh_token()? {
      Some(bearer) => bearer,
      None => {
        info!("No full WorldLabs refresh");
        tokio::time::sleep(Duration::from_millis(30_000)).await;
        continue;
      }
    };

    let local_tasks = list_tasks_by_provider_and_status(ListTasksByProviderAndStatusArgs {
      db: task_database.get_connection(),
      provider: GenerationSource::WorldLabs,
      task_statuses: &TASK_DATABASE_PENDING_STATUSES,
    }).await?;

    poll_worldlabs_tasks(
      app_handle,
      app_data_root,
      app_preferences,
      task_database,
      &world_labs_cookies,
      &world_labs_bearer,
      &world_labs_refresh,
      maybe_storyteller_creds.as_ref(),
      local_tasks,
    ).await?;

    tokio::time::sleep(Duration::from_millis(2_000)).await;
  }
}

async fn poll_worldlabs_tasks(
  app_handle: &AppHandle,
  app_data_root: &AppDataRoot,
  app_preferences: &AppPreferencesManager,
  task_database: &TaskDatabase,
  world_labs_cookies: &WorldLabsCookies,
  world_labs_bearer: &WorldLabsBearerToken,
  _world_labs_refresh: &WorldLabsRefreshToken,
  maybe_storyteller_creds: Option<&StorytellerCredentialSet>,
  local_tasks: TaskList,
) -> AnyhowResult<()> {
  let local_tasks = local_tasks.tasks;

  if local_tasks.is_empty() {
    return Ok(())
  }

  info!("WorldLabs tasks waiting: {:?}", local_tasks.len());

  // Map of WorldLabs World ID to Local Task.
  let local_tasks_by_world_labs_world_id = local_tasks.iter()
      .filter_map(|task| {
        if let Some(provider_job_id) = &task.provider_job_id {
          Some((provider_job_id.clone(), task.clone()))
        } else {
          None
        }
      })
      .collect::<HashMap<String, Task>>();

  for (world_id, local_task) in local_tasks_by_world_labs_world_id.iter() {
    let world_id = WorldObjectId(world_id.to_string());

    let poll_world_response = poll_world_status(PollWorldStatusArgs {
      cookies: &world_labs_cookies,
      bearer_token: &world_labs_bearer,
      world_id: &world_id,
      request_timeout: None,
    }).await?;

    if !poll_world_response.is_complete {
      tokio::time::sleep(Duration::from_millis(2_000)).await;
      continue;
    }

    let spz_url = match poll_world_response.spz_splat_url {
      Some(url) => url,
      None => {
        error!("No spz splat URL despite being marked complete");
        continue;
      }
    };

    let result = complete_spz_splat(
      app_handle,
      app_data_root,
      app_preferences,
      task_database,
      maybe_storyteller_creds,
      local_task,
      &spz_url,
    ).await;

    if let Err(err) = result {
      error!("Failed to complete WorldLabs task {}: {:?}", local_task.id.as_str(), err);
    }
  }

  tokio::time::sleep(Duration::from_millis(5_000)).await;

  Ok(())
}

/// Download the finished splat to the temp dir, then hand it to the shared
/// completion routine.
async fn complete_spz_splat(
  app_handle: &AppHandle,
  app_data_root: &AppDataRoot,
  app_preferences: &AppPreferencesManager,
  task_database: &TaskDatabase,
  maybe_storyteller_creds: Option<&StorytellerCredentialSet>,
  local_task: &Task,
  spz_url: &str,
) -> AnyhowResult<bool> {
  info!("Downloading generated spz splat ...");

  let download_path = download_spz(spz_url, app_data_root).await?;

  complete_task_with_local_files(CompleteTaskArgs {
    app_handle,
    app_data_root,
    app_preferences,
    task_database,
    maybe_storyteller_creds,
    task: local_task,
    generation_provider: GenerationSource::WorldLabs,
    media_class: TaskMediaFileClass::Splat,
    prompt: CompletionPrompt::None,
    fallback_model_slug: MARBLE_FALLBACK_MODEL_SLUG,
    local_files: &[download_path],
  }).await
}

async fn download_spz(
  spz_url: &str,
  app_data_root: &AppDataRoot,
) -> AnyhowResult<PathBuf> {
  info!("Downloading splat from URL: {}", spz_url);

  let response = reqwest::get(spz_url).await?;
  let image_bytes = response.bytes().await?;

  let uuid = generate_random_uuid();
  let extension_without_period = "ceramic.spz";

  let tempdir = app_data_root.temp_dir().path();
  let download_filename = format!("{}.{}", uuid, extension_without_period);
  let download_path = tempdir.join(download_filename);

  info!("Writing to path: {:?}", download_path);

  let mut file = File::create(&download_path)?;
  file.write_all(&image_bytes)?;

  Ok(download_path)
}

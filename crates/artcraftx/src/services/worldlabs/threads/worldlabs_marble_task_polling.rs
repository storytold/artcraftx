use artcraft_client::utils::api_host::ApiHost;
use crate::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::events::generation_events::common::{GenerationAction, GenerationServiceProvider};
use crate::events::generation_events::generation_complete_event::GenerationCompleteEvent;
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::data_dir::subdirectory::trait_data_subdir::DataSubdir;
use crate::state::database::task_database::TaskDatabase;
use crate::utils::task_database_pending_statuses::TASK_DATABASE_PENDING_STATUSES;
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;
use crate::services::worldlabs::state::worldlabs_credential_manager::WorldlabsCredentialManager;
use artcraft_client::api_defs::utils::media_links_to_thumbnail_template::media_links_to_thumbnail_template;
use sqlite_identifiers::enums::generation_provider::GenerationProvider;
use sqlite_identifiers::enums::task_media_file_class::TaskMediaFileClass;
use errors::AnyhowResult;
use uuid_utils::uuid::generate_random_uuid;
use log::{error, info};
use sqlite_database::queries::read::list_tasks_by_provider_and_status::{list_tasks_by_provider_and_status, ListTasksByProviderAndStatusArgs, TaskList};
use sqlite_database::queries::task::Task;
use sqlite_database::queries::update::update_successful_task_status_with_metadata::{update_successful_task_status_with_metadata, UpdateSuccessfulTaskArgs};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use artcraft_client::credentials::storyteller_credential_set::StorytellerCredentialSet;
use artcraft_client::endpoints::media_files::get_media_file::get_media_file;
use artcraft_client::endpoints::media_files::legacy_upload_media_file_from_file::{legacy_upload_media_file_from_file, LegacyUploadMediaFileFromFileArgs};
use artcraft_client::error::api_error::ApiError;
use artcraft_client::error::storyteller_error::StorytellerError;
use tauri::AppHandle;
use worldlabs_consumer_client::api::api_types::world_id::WorldObjectId;
use worldlabs_consumer_client::api::requests::worlds::poll_world_status::{poll_world_status, PollWorldStatusArgs};
use worldlabs_consumer_client::credentials::world_labs_bearer_token::WorldLabsBearerToken;
use worldlabs_consumer_client::credentials::world_labs_cookies::WorldLabsCookies;
use worldlabs_consumer_client::credentials::worldlabs_refresh_token::WorldLabsRefreshToken;

pub async fn worldlabs_marble_task_polling(
  app_handle: AppHandle,
  app_data_root: AppDataRoot,
  task_database: TaskDatabase,
  creds: WorldlabsCredentialManager,
  storyteller_creds_manager: StorytellerCredentialManager,
) -> ! {
  loop {
    let res = polling_loop(
      &app_handle,
      &app_data_root,
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
  task_database: &TaskDatabase,
  worldlabs_creds: &WorldlabsCredentialManager,
  storyteller_creds_manager: &StorytellerCredentialManager,
) -> AnyhowResult<()> {
  loop {
    if !worldlabs_creds.do_task_polling()? {
      tokio::time::sleep(Duration::from_millis(10_000)).await;
      continue;
    }

    // TODO: Graceful wait, fix this long function body
    let storyteller_creds = match storyteller_creds_manager.get_credentials()? {
      Some(creds) => creds,
      None => {
        error!("No Storyteller credentials found. Cannot proceed with WorldLabs polling.");
        tokio::time::sleep(Duration::from_millis(5_000)).await;
        continue;
      }
    };

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
      provider: GenerationProvider::WorldLabs,
      task_statuses: &TASK_DATABASE_PENDING_STATUSES,
    }).await?;

    poll_grok_tasks(
      app_handle,
      app_data_root,
      task_database,
      &world_labs_cookies,
      &world_labs_bearer,
      &world_labs_refresh,
      &storyteller_creds,
      local_tasks,
    ).await?;

    tokio::time::sleep(Duration::from_millis(2_000)).await;
  }
}

async fn poll_grok_tasks(
  app_handle: &AppHandle,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
  world_labs_cookies: &WorldLabsCookies,
  world_labs_bearer: &WorldLabsBearerToken,
  _world_labs_refresh: &WorldLabsRefreshToken,
  storyteller_creds: &StorytellerCredentialSet,
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

    upload_spz_splat(
      &app_handle,
      app_data_root,
      task_database,
      &storyteller_creds,
      &local_task,
      &spz_url,
    ).await?;
  }

  tokio::time::sleep(Duration::from_millis(5_000)).await;

  Ok(())
}

async fn upload_spz_splat(
  app_handle: &AppHandle,
  app_data_root: &AppDataRoot,
  task_database: &TaskDatabase,
  storyteller_creds: &StorytellerCredentialSet,
  local_task: &Task,
  spz_url: &str,
) -> AnyhowResult<()> {

  let mut maybe_primary_media_file_token = None;

  info!("Downloading generated spz splat ...");

  let spz_download_filename = download_spz(spz_url, app_data_root).await?;

  let mut wait_delay = 0;

  loop {
    info!("Uploading to backend...");

    // TODO: media_files.origin_category
    // TODO: media_files.maybe_prompt_token
    // TODO: media_files.maybe_generation_provider
    // TODO: media_files.maybe_origin_model_type
    // TODO: media_files.maybe_origin_model_token (sref?)
    // TODO: media_files.maybe_batch_token
    // TODO: media_files.is_user_upload

    // TODO: batch_generations.token
    // TODO: batch_generations.entity_type
    // TODO: batch_generations.entity_token

    let result = legacy_upload_media_file_from_file(LegacyUploadMediaFileFromFileArgs {
      api_host: &ApiHost::Storyteller,
      maybe_creds: Some(&storyteller_creds),
      path: &spz_download_filename,
      maybe_generation_provider: Some(GenerationProvider::WorldLabs),
    }).await;

    match result {
      Ok(result) => {
        info!("Successfully uploaded to backend: {:?}", result.media_file_token);
        if maybe_primary_media_file_token.is_none() {
          maybe_primary_media_file_token = Some(result.media_file_token);
        }
        break;
      },
      Err(StorytellerError::Api(ApiError::TooManyRequests(_))) => {
        error!("Too many requests, retrying upload after delay...");
        // If we hit a rate limit, we can retry after a short delay.
        wait_delay += 10;
        if wait_delay > 60 {
          wait_delay = 60;
        }
        tokio::time::sleep(Duration::from_secs(wait_delay)).await;
        continue; // Retry the upload.
      }
      Err(err) => {
        error!("Failed to upload to backend: {:?}", err);
        return Err(err.into())
      },
    }
  } // End loop

  let mut maybe_cdn_url = None;
  let mut maybe_thumbnail_url_template = None;

  if let Some(media_file_token) = maybe_primary_media_file_token.as_ref() {
    info!("Looking up file to grab CDN and thumbnail URLs: {:?} ...", media_file_token);

    let lookup_result = get_media_file(
      &ApiHost::Storyteller,
      media_file_token,
    ).await;
    match lookup_result {
      Ok(response) => {
        maybe_cdn_url = Some(response.media_file.media_links.cdn_url.to_string());
        maybe_thumbnail_url_template = media_links_to_thumbnail_template(&response.media_file.media_links)
            .map(|s| s.to_string());
      }
      Err(err) => {
        error!("Failed to look up media file after upload: {:?} (failing open)", err);
      }
    }
  }

  let updated = update_successful_task_status_with_metadata(UpdateSuccessfulTaskArgs {
    db: task_database.get_connection(),
    task_id: &local_task.id,
    maybe_batch_token: None,
    maybe_primary_media_file_token: maybe_primary_media_file_token.as_ref(),
    maybe_primary_media_file_class: Some(TaskMediaFileClass::Dimensional),
    maybe_primary_media_file_thumbnail_url_template: maybe_thumbnail_url_template.as_deref(),
    maybe_primary_media_file_cdn_url: maybe_cdn_url.as_deref(),
  }).await?;

  if !updated {
    return Ok(()); // If anything breaks with queries, don't spam events.
  }

  let event = GenerationCompleteEvent {
    //media_file_token: result.media_file_token,
    action: Some(GenerationAction::GenerateGaussian),
    service: GenerationServiceProvider::WorldLabs,
    model: None,
  };

  if let Err(err) = event.send(&app_handle) {
    error!("Failed to send GenerationCompleteEvent: {:?}", err); // Fail open
  }

  //let result = maybe_handle_text_to_image_complete_event(
  //  app_handle,
  //  app_env_configs,
  //  Some(storyteller_creds),
  //  local_task,
  //  &batch_token,
  //).await;

  //if let Err(err) = result {
  //  error!("Failed to send text-to-image complete event: {:?}", err);
  //}

  Ok(())
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

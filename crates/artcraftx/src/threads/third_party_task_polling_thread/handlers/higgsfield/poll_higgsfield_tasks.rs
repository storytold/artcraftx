use std::collections::HashMap;

use higgsfield_client::endpoints::jobs::job_status::JobStatusResponse;
use higgsfield_client::session::higgsfield_session::HiggsfieldSession;
use higgsfield_client::types::ids::JobId;
use higgsfield_client::types::job_status::JobStatus;
use log::{error, info, warn};
use sqlite_database::queries::task::Task;
use tauri::AppHandle;

use crate::commands::generate::common::higgsfield_generation::split_higgsfield_job_ids;
use crate::services::storyteller::state::storyteller_credential_manager::StorytellerCredentialManager;
use crate::state::app_preferences::app_preferences_manager::AppPreferencesManager;
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::state::database::task_database::TaskDatabase;
use crate::threads::third_party_task_polling_thread::handlers::higgsfield::handle_higgsfield_complete::handle_higgsfield_complete;
use crate::threads::third_party_task_polling_thread::handlers::higgsfield::handle_higgsfield_failure::handle_higgsfield_failure;
use crate::threads::third_party_task_polling_thread::handlers::higgsfield::higgsfield_poll_sessions::HiggsfieldPollSessions;

/// Check every pending Higgsfield task. Jobs are looked up through each
/// stored Higgsfield account in turn (a job only answers on the account that
/// created it); the first account that knows the job set is used.
pub async fn poll_higgsfield_tasks(
  app_handle: &AppHandle,
  app_data_root: &AppDataRoot,
  app_preferences: &AppPreferencesManager,
  task_database: &TaskDatabase,
  storyteller_creds_manager: &StorytellerCredentialManager,
  sessions: &mut HiggsfieldPollSessions,
  tasks: &[&Task],
) {
  let sessions = sessions.refresh(app_data_root);
  if sessions.is_empty() {
    warn!("[HiggsfieldPolling] {} task(s) pending but no Higgsfield account is stored; skipping", tasks.len());
    return;
  }

  for task in tasks {
    let result = poll_single_task(
      app_handle,
      app_data_root,
      app_preferences,
      task_database,
      storyteller_creds_manager,
      &sessions,
      task,
    ).await;
    if let Err(err) = result {
      error!("[HiggsfieldPolling] Error processing task {}: {:?}", task.id.as_str(), err);
    }
  }
}

/// What the batch status call said about one task's jobs.
enum JobSetProgress {
  /// At least one job is still running.
  Pending,
  /// Every job reached a terminal state.
  Done(Vec<JobOutcome>),
}

struct JobOutcome {
  job_id: JobId,
  status: JobStatus,
}

async fn poll_single_task(
  app_handle: &AppHandle,
  app_data_root: &AppDataRoot,
  app_preferences: &AppPreferencesManager,
  task_database: &TaskDatabase,
  storyteller_creds_manager: &StorytellerCredentialManager,
  sessions: &[(String, HiggsfieldSession)],
  task: &Task,
) -> anyhow::Result<()> {
  let job_ids: Vec<JobId> = task.provider_job_id.as_deref()
      .map(split_higgsfield_job_ids)
      .unwrap_or_default()
      .into_iter()
      .map(JobId::new)
      .collect();

  if job_ids.is_empty() {
    warn!("[HiggsfieldPolling] Task {} has no job ids; marking as failed", task.id.as_str());
    handle_higgsfield_failure(app_handle, task_database, task, "No Higgsfield job ids to check").await;
    return Ok(());
  }

  let Some((session, progress)) = find_job_set(sessions, &job_ids, task).await else {
    // No account knows the jobs. Leave the task pending: the account may be
    // temporarily unreachable (expired session, network) and come back.
    return Ok(());
  };

  let outcomes = match progress {
    JobSetProgress::Pending => return Ok(()),
    JobSetProgress::Done(outcomes) => outcomes,
  };

  // Fetch the full record (result URLs, prompt) of every finished job.
  let mut finished: Vec<JobStatusResponse> = Vec::new();
  let mut failures: Vec<String> = Vec::new();
  for outcome in &outcomes {
    if !outcome.status.is_success() {
      failures.push(format!("{} ended {}", outcome.job_id, outcome.status));
      continue;
    }
    match session.job_status(&outcome.job_id).await {
      Ok(job) if job.result_url().is_some() => finished.push(job),
      Ok(job) => failures.push(format!("{} completed without a result URL", job.id)),
      Err(err) => {
        // Transient: try again next iteration rather than failing the task.
        warn!("[HiggsfieldPolling] Could not fetch job {} for task {}: {}", outcome.job_id, task.id.as_str(), err);
        return Ok(());
      }
    }
  }

  if finished.is_empty() {
    let reason = if failures.is_empty() { "Higgsfield produced no output".to_string() } else { failures.join("; ") };
    handle_higgsfield_failure(app_handle, task_database, task, &reason).await;
    return Ok(());
  }

  if !failures.is_empty() {
    warn!(
      "[HiggsfieldPolling] Task {}: {} of {} job(s) failed ({}); delivering the rest",
      task.id.as_str(), failures.len(), outcomes.len(), failures.join("; "),
    );
  }

  handle_higgsfield_complete(
    app_handle,
    app_data_root,
    app_preferences,
    task_database,
    storyteller_creds_manager,
    task,
    &finished,
  ).await;

  Ok(())
}

/// Ask each account for the jobs' status until one knows them.
async fn find_job_set<'a>(
  sessions: &'a [(String, HiggsfieldSession)],
  job_ids: &[JobId],
  task: &Task,
) -> Option<(&'a HiggsfieldSession, JobSetProgress)> {
  for (credential_id, session) in sessions {
    let batch = match session.job_status_batch(job_ids).await {
      Ok(batch) => batch,
      Err(err) => {
        warn!(
          "[HiggsfieldPolling] Status check failed for task {} on credential {}: {}{}",
          task.id.as_str(), credential_id, err,
          if err.needs_browser_reauth() { " (log into Higgsfield again)" } else { "" },
        );
        continue;
      }
    };

    // An account that created none of these jobs reports them all missing.
    if batch.items.is_empty() {
      continue;
    }

    let statuses: HashMap<&JobId, &JobStatus> = batch.items.iter().map(|item| (&item.id, &item.status)).collect();
    let mut outcomes = Vec::with_capacity(job_ids.len());
    for job_id in job_ids {
      let status = match statuses.get(job_id) {
        Some(status) => (*status).clone(),
        None if batch.missing.contains(job_id) => JobStatus::Other("missing".to_string()),
        None => return Some((session, JobSetProgress::Pending)),
      };
      if !status.is_terminal() && !batch.missing.contains(job_id) {
        info!("[HiggsfieldPolling] Task {} job {} is {}", task.id.as_str(), job_id, status);
        return Some((session, JobSetProgress::Pending));
      }
      outcomes.push(JobOutcome { job_id: job_id.clone(), status });
    }
    return Some((session, JobSetProgress::Done(outcomes)));
  }
  None
}

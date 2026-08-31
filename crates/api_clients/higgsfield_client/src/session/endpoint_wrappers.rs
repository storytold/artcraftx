//! Every endpoint as a [`HiggsfieldSession`] method: the session supplies a
//! fresh bearer token and retries once on `401`.

use crate::endpoints::generate::image::gpt_image_2::{gpt_image_2, GptImage2Args, GptImage2Request};
use crate::endpoints::generate::image::nano_banana_pro::{nano_banana_pro, NanoBananaProArgs, NanoBananaProRequest};
use crate::endpoints::jobs::job_status::{job_status, JobStatusArgs, JobStatusRequest, JobStatusResponse};
use crate::endpoints::jobs::job_status_batch::{job_status_batch, JobStatusBatchArgs, JobStatusBatchRequest, JobStatusBatchResponse};
use crate::endpoints::user::user_data::{user_data, UserDataArgs, UserDataRequest, UserDataResponse};
use crate::endpoints::user::user_profile::{user_profile, UserProfileArgs, UserProfileRequest, UserProfileResponse};
use crate::error::higgsfield_error::HiggsfieldError;
use crate::session::higgsfield_session::HiggsfieldSession;
use crate::types::enqueue_jobs_response::EnqueueJobsResponse;
use crate::types::ids::JobId;

impl HiggsfieldSession {
  /// Enqueue a Nano Banana Pro image job.
  pub async fn nano_banana_pro(&self, request: NanoBananaProRequest) -> Result<EnqueueJobsResponse, HiggsfieldError> {
    self.with_auth(|auth| {
      let request = request.clone();
      async move { nano_banana_pro(NanoBananaProArgs { request, auth: &auth, host: self.api_host() }).await }
    }).await
  }

  /// Enqueue a GPT Image job.
  pub async fn gpt_image_2(&self, request: GptImage2Request) -> Result<EnqueueJobsResponse, HiggsfieldError> {
    self.with_auth(|auth| {
      let request = request.clone();
      async move { gpt_image_2(GptImage2Args { request, auth: &auth, host: self.api_host() }).await }
    }).await
  }

  /// Full state of one job, including result URLs once complete.
  pub async fn job_status(&self, job_id: &JobId) -> Result<JobStatusResponse, HiggsfieldError> {
    self.with_auth(|auth| {
      let request = JobStatusRequest { job_id: job_id.clone() };
      async move { job_status(JobStatusArgs { request, auth: &auth, host: self.api_host() }).await }
    }).await
  }

  /// Lightweight status for many jobs.
  pub async fn job_status_batch(&self, job_ids: &[JobId]) -> Result<JobStatusBatchResponse, HiggsfieldError> {
    self.with_auth(|auth| {
      let request = JobStatusBatchRequest { ids: job_ids.to_vec() };
      async move { job_status_batch(JobStatusBatchArgs { request, auth: &auth, host: self.api_host() }).await }
    }).await
  }

  /// The account: plan, credits, workspace. Also the cheapest session check.
  pub async fn user_data(&self) -> Result<UserDataResponse, HiggsfieldError> {
    self.with_auth(|auth| async move {
      user_data(UserDataArgs { request: UserDataRequest, auth: &auth, host: self.api_host() }).await
    }).await
  }

  /// The public profile.
  pub async fn user_profile(&self) -> Result<UserProfileResponse, HiggsfieldError> {
    self.with_auth(|auth| async move {
      user_profile(UserProfileArgs { request: UserProfileRequest, auth: &auth, host: self.api_host() }).await
    }).await
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::client::clerk_host::ClerkHost;
  use crate::client::higgsfield_host::HiggsfieldHost;
  use crate::error::higgsfield_client_error::HiggsfieldClientError;
  use crate::types::gpt_image_quality::GptImageQuality;
  use crate::types::image_aspect_ratio::ImageAspectRatio;
  use crate::types::image_resolution::ImageResolution;

  /// A session that can't reach anything, but has a fresh seed token so the
  /// wrappers get past auth and into request validation.
  fn offline_session() -> HiggsfieldSession {
    use crate::credentials::clerk_session_token::tests::fake_clerk_token;
    let token = fake_clerk_token((chrono::Utc::now() + chrono::Duration::hours(1)).timestamp());
    HiggsfieldSession::from_cookie_header("__client=x")
        .with_hosts(HiggsfieldHost::Custom("http://127.0.0.1:9".into()), ClerkHost::Custom("http://127.0.0.1:9".into()))
        .with_initial_token(token)
  }

  #[tokio::test]
  async fn wrappers_apply_request_validation() {
    let session = offline_session();

    let mut request = NanoBananaProRequest::text_to_image("p", ImageAspectRatio::Square1x1, ImageResolution::OneK);
    request.batch_size = 0;
    let err = session.nano_banana_pro(request).await.unwrap_err();
    assert!(matches!(err, HiggsfieldError::Client(HiggsfieldClientError::InvalidRequest(_))));

    let request = GptImage2Request::text_to_image("", ImageAspectRatio::Square1x1, GptImageQuality::Low, ImageResolution::OneK);
    let err = session.gpt_image_2(request).await.unwrap_err();
    assert!(matches!(err, HiggsfieldError::Client(HiggsfieldClientError::InvalidRequest(_))));

    let err = session.job_status_batch(&[]).await.unwrap_err();
    assert!(matches!(err, HiggsfieldError::Client(HiggsfieldClientError::InvalidRequest(_))));
  }

  // ── Live (ignored: needs captured cookies; the enqueue spends credits) ──

  #[tokio::test]
  #[ignore]
  async fn live_session_user_data() -> anyhow::Result<()> {
    use crate::test_utils::higgsfield_test_secrets::load_higgsfield_test_session;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();

    let session = load_higgsfield_test_session()?;
    let user = session.user_data().await.map_err(|err| anyhow::anyhow!("{err}"))?;
    let profile = session.user_profile().await.map_err(|err| anyhow::anyhow!("{err}"))?;
    println!("User {} ({}) plan={:?} credits={:?}", user.id, profile.username, user.plan_type, user.subscription_credits);
    Ok(())
  }

  #[tokio::test]
  #[ignore]
  async fn live_session_generate_and_wait() -> anyhow::Result<()> {
    use crate::session::wait_for_job::WaitForJobOptions;
    use crate::test_utils::higgsfield_test_secrets::load_higgsfield_test_session;
    use crate::test_utils::setup_test_logging::setup_test_logging;
    setup_test_logging();

    let session = load_higgsfield_test_session()?;
    let enqueued = session.nano_banana_pro(
      NanoBananaProRequest::text_to_image("a dinosaur on a skateboard", ImageAspectRatio::Portrait3x4, ImageResolution::OneK),
    ).await.map_err(|err| anyhow::anyhow!("{err}"))?;

    let job_id = enqueued.job_ids().into_iter().next().unwrap();
    let job = session.wait_for_job(&job_id, WaitForJobOptions::default()).await.map_err(|err| anyhow::anyhow!("{err}"))?;
    println!("Job {} finished: {:?}", job.id, job.result_url());
    assert!(job.result_url().is_some());
    Ok(())
  }
}

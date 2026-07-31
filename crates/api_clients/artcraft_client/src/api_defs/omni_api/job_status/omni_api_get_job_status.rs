use serde_derive::{Deserialize, Serialize};

use tokens::tokens::generic_inference_jobs::InferenceJobToken;

use crate::api_defs::omni_api::job_status::omni_api_job_status_payload::OmniApiJobStatusPayload;

/// Path params for `GET /v1/omni_api/job_status/job/{token}`.
#[derive(Deserialize)]
pub struct OmniApiGetJobStatusPathInfo {
  pub token: InferenceJobToken,
}

/// Success body for `GET /v1/omni_api/job_status/job/{token}`.
#[derive(Serialize, Deserialize)]
pub struct OmniApiGetJobStatusSuccessResponse {
  pub success: bool,
  pub state: OmniApiJobStatusPayload,
}

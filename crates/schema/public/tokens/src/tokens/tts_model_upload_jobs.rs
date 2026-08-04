use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::LegacyTokenPrefix;

/// Primary key for the `tts_model_upload_jobs` table.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct TtsModelUploadJobToken(pub String);

impl_string_token!(TtsModelUploadJobToken);
impl_crockford_generator!(TtsModelUploadJobToken, 32usize, LegacyTokenPrefix::TtsModelUploadJob, CrockfordLower);

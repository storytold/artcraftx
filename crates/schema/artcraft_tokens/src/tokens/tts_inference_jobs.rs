use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::LegacyTokenPrefix;

/// Primary key for the `tts_inference_jobs` table.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct TtsInferenceJobToken(pub String);

impl_string_token!(TtsInferenceJobToken);
impl_crockford_generator!(TtsInferenceJobToken, 32usize, LegacyTokenPrefix::TtsInferenceJob, CrockfordLower);

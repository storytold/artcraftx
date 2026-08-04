use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::LegacyTokenPrefix;

/// Primary key for the `tts_results` table.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct TtsResultToken(pub String);

impl_string_token!(TtsResultToken);
impl_crockford_generator!(TtsResultToken, 32usize, LegacyTokenPrefix::TtsResult, CrockfordLower);

use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for "generic" inference jobs.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub struct InferenceJobToken(String);

impl_crockford_generator!(InferenceJobToken, 32usize, TokenPrefix::InferenceJob, CrockfordLower);
impl_string_token!(InferenceJobToken);

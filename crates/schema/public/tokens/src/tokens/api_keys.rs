use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for api_keys.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize, Default)]
pub struct ApiKeyToken(pub String);
impl_string_token!(ApiKeyToken);
impl_crockford_generator!(ApiKeyToken, 32usize, TokenPrefix::ApiKey, CrockfordLower);

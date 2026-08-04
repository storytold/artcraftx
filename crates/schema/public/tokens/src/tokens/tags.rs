use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for Prompts
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct TagToken(pub String);

impl_crockford_generator!(TagToken, 32usize, TokenPrefix::Tag, CrockfordLower);
impl_string_token!(TagToken);

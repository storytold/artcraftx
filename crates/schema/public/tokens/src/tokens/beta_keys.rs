use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for Audit Logs.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct BetaKeyToken(pub String);

impl_crockford_generator!(BetaKeyToken, 32usize, TokenPrefix::BetaKey, CrockfordLower);
impl_string_token!(BetaKeyToken);

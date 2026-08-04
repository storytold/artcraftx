use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for Audit Logs.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct BrowserSessionLogToken(pub String);

impl_crockford_generator!(BrowserSessionLogToken, 32usize, TokenPrefix::BrowserSessionLog, CrockfordLower);
impl_string_token!(BrowserSessionLogToken);

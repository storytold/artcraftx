use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for user_spend_events
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct UserSpendEventToken(pub String);

impl_string_token!(UserSpendEventToken);
impl_crockford_generator!(UserSpendEventToken, 32usize, TokenPrefix::UserSpendEvent, CrockfordLower);

use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for users.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub struct UserToken(pub String);
impl_string_token!(UserToken);
// NB: Older user tokens were under this regime: 15 characters, "U:" prefix, Crockford Upper.
impl_crockford_generator!(UserToken, 18usize, TokenPrefix::User, CrockfordLower);

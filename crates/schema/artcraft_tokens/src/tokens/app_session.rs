use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for users.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub struct AppSessionToken(pub String);
impl_string_token!(AppSessionToken);
impl_crockford_generator!(AppSessionToken, 32usize, TokenPrefix::AppSession, CrockfordMixed);

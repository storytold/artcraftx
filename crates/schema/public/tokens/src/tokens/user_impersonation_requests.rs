use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for User Impersonation Requests
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct UserImpersonationRequestToken(pub String);

impl_string_token!(UserImpersonationRequestToken);
impl_crockford_generator!(UserImpersonationRequestToken, 32usize, TokenPrefix::UserImpersonationRequest, CrockfordLower);

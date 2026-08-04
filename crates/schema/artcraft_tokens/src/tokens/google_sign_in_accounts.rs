use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for google_sign_in_accounts
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub struct GoogleSignInAccountToken(String);

impl_crockford_generator!(GoogleSignInAccountToken, 32usize, TokenPrefix::GoogleSignInAccount, CrockfordLower);
impl_string_token!(GoogleSignInAccountToken);

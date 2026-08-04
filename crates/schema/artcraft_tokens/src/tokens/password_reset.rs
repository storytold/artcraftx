use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for `password_reset`s
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PasswordResetToken(pub String);

impl_string_token!(PasswordResetToken);
impl_crockford_generator!(PasswordResetToken, 32usize, TokenPrefix::PasswordReset, CrockfordMixed);

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for user referral codes.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub struct UserReferralCodeToken(pub String);
impl_string_token!(UserReferralCodeToken);
impl_crockford_generator!(UserReferralCodeToken, 18usize, TokenPrefix::UserReferralCode, CrockfordLower);

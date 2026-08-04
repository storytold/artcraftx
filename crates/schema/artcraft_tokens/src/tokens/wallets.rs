use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for wallets
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct WalletToken(pub String);

impl_string_token!(WalletToken);
impl_crockford_generator!(WalletToken, 32usize, TokenPrefix::Wallet, CrockfordLower);

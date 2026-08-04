use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for wallet_ledger_entries
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct WalletLedgerEntryToken(pub String);

impl_string_token!(WalletLedgerEntryToken);
impl_crockford_generator!(WalletLedgerEntryToken, 32usize, TokenPrefix::WalletLedgerEntry, CrockfordLower);

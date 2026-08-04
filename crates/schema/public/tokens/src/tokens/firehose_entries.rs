use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::LegacyTokenPrefix;

/// Primary key for the `firehose_entries` table.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct FirehoseEntryToken(pub String);

impl_string_token!(FirehoseEntryToken);
impl_crockford_generator!(FirehoseEntryToken, 32usize, LegacyTokenPrefix::FirehoseEntry, CrockfordLower);

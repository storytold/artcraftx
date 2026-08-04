use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// A non-unique event token for the `debug_logs` table.
/// Multiple log rows can share the same event token.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct DebugLogEventToken(pub String);

impl_string_token!(DebugLogEventToken);
impl_crockford_generator!(DebugLogEventToken, 32usize, TokenPrefix::DebugLogEvent, CrockfordLower);

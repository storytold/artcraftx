use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::LegacyTokenPrefix;

/// primary key token for the `twitch_event_rules` table (this is deprecated)
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct TwitchEventRuleToken(pub String);

impl_string_token!(TwitchEventRuleToken);
impl_crockford_generator!(TwitchEventRuleToken, 32usize, LegacyTokenPrefix::TwitchEventRule, CrockfordLower);

use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// A rare instance that is *not* a primary key.
/// These are generated as cookies to track users for a better logged out experience and for analytics.
/// We use these as indices into several columns (ML result types, uploads, etc.)
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize, Default)]
pub struct AnonymousVisitorTrackingToken(pub String);

impl_crockford_generator!(AnonymousVisitorTrackingToken, 32usize, TokenPrefix::AnonymousVisitorTracking, CrockfordLower);
impl_string_token!(AnonymousVisitorTrackingToken);

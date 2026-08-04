use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for email_sender_jobs
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct EmailSenderJobToken(pub String);

impl_string_token!(EmailSenderJobToken);
impl_crockford_generator!(EmailSenderJobToken, 32usize, TokenPrefix::EmailSenderJob, CrockfordLower);

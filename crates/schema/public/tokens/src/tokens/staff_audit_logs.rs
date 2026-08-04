use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for Staff Audit Logs
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct StaffAuditLogToken(pub String);

impl_string_token!(StaffAuditLogToken);
impl_crockford_generator!(StaffAuditLogToken, 32usize, TokenPrefix::StaffAuditLog, CrockfordLower);

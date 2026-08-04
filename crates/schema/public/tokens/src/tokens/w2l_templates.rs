use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::LegacyTokenPrefix;

/// The primary key for W2L templates.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct W2lTemplateToken(pub String);

impl_string_token!(W2lTemplateToken);
impl_crockford_generator!(W2lTemplateToken, 32usize, LegacyTokenPrefix::W2lTemplate, CrockfordLower);

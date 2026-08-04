use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::LegacyTokenPrefix;

/// The primary key for model categories.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct ModelCategoryToken(pub String);

impl_string_token!(ModelCategoryToken);
impl_crockford_generator!(ModelCategoryToken, 15usize, LegacyTokenPrefix::ModelCategory, CrockfordLower);

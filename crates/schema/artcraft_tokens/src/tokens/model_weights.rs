use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for the  "model_weights" table.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct ModelWeightToken(pub String);

impl_crockford_generator!(ModelWeightToken, 32usize, TokenPrefix::ModelWeight, CrockfordLower);
impl_string_token!(ModelWeightToken);

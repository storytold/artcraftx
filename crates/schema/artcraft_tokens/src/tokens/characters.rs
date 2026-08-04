use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for Characters
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct CharacterToken(pub String);

impl_string_token!(CharacterToken);
impl_crockford_generator!(CharacterToken, 32usize, TokenPrefix::Character, CrockfordLower);

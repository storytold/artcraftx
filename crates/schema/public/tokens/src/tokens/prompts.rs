use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for Prompts
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "database", derive(sqlx::Type))]
#[cfg_attr(feature = "database", sqlx(transparent))]
pub struct PromptToken(pub String);

impl_crockford_generator!(PromptToken, 32usize, TokenPrefix::Prompt, CrockfordLower);
impl_mysql_token_from_row!(PromptToken);
impl_string_token!(PromptToken);

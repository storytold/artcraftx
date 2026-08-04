use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TokenPrefix;

/// The primary key for folders.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize, Default)]
pub struct FolderToken(pub String);
impl_string_token!(FolderToken);
impl_crockford_generator!(FolderToken, 32usize, TokenPrefix::Folder, CrockfordLower);

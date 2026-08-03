use std::fmt::Debug;

use serde::Deserialize;
use serde::Serialize;

use crate::prefixes::TauriTokenPrefix;

/// The stable identifier for a stored credential in the Tauri desktop app.
///
/// Unlike most tokens, this is not a database primary key: it lives inside
/// each credential TOML file in the app's credentials directory. It is hidden
/// from users but serves as the effective primary identifier for a credential
/// (file names can be freely renamed).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "database", derive(sqlx::Type))]
#[cfg_attr(feature = "database", sqlx(transparent))]
pub struct CredentialToken(pub String);

impl_string_token!(CredentialToken);
impl_crockford_generator!(CredentialToken, 32usize, TauriTokenPrefix::Credential, CrockfordLower);

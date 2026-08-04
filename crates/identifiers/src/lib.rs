//! Strongly-typed identifiers for entities that are NOT database rows
//! (credential files, and whatever comes next).
//!
//! An id is `{prefix}_{26-char lowercase Crockford ULID}` (`credential_01j9…`):
//! time-ordered, globally unique, and — crucially — TYPED: a `CredentialId`
//! can never be passed where some other id is expected. Never use bare
//! `String` ids.
//!
//! Database primary keys live in the `tokens` crate; this crate is for
//! identifiers minted and stored outside a database.

use std::fmt;

// ─────────────────────────── Machinery ───────────────────────────

/// Mint a fresh id string: `{prefix}_{26-char lowercase Crockford ULID}`.
pub fn mint(prefix: &str) -> String {
  format!("{prefix}_{}", ulid::Ulid::new().to_string().to_lowercase())
}

/// Error parsing a prefixed id from an untrusted string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseIdError {
  /// The string didn't start with `{prefix}_`.
  WrongPrefix { expected: &'static str, got: String },
  /// The part after the prefix wasn't a 26-char Crockford ULID.
  BadUlid { got: String },
}

impl fmt::Display for ParseIdError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::WrongPrefix { expected, got } => {
        write!(f, "expected id with prefix '{expected}_', got '{got}'")
      }
      Self::BadUlid { got } => write!(f, "id '{got}' does not end in a 26-char ULID"),
    }
  }
}

impl std::error::Error for ParseIdError {}

/// Validate that `s` is a well-formed `{prefix}_{ulid}` id.
pub fn validate(s: &str, prefix: &'static str) -> Result<(), ParseIdError> {
  let rest = s
    .strip_prefix(prefix)
    .and_then(|r| r.strip_prefix('_'))
    .ok_or_else(|| ParseIdError::WrongPrefix { expected: prefix, got: s.to_string() })?;

  // A Crockford ULID is 26 chars of lowercase base32 (0-9 a-z after lowercasing).
  if rest.len() != 26 || !rest.bytes().all(|b| b.is_ascii_alphanumeric()) {
    return Err(ParseIdError::BadUlid { got: s.to_string() });
  }
  Ok(())
}

/// Define a strongly-typed id newtype: `define_id!(pub struct CredentialId => "credential");`.
///
/// The generated type is a transparent newtype over `String` that:
///   - `generate()`s a fresh `{prefix}_{ulid}`,
///   - parses/validates via `FromStr`,
///   - `Display`s / `Serialize`s as the bare string.
#[macro_export]
macro_rules! define_id {
  ($(#[$doc:meta])* $vis:vis struct $name:ident => $prefix:literal) => {
    $(#[$doc])*
    #[derive(
      Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
      ::serde::Serialize, ::serde::Deserialize,
    )]
    #[serde(transparent)]
    $vis struct $name(String);

    impl $name {
      /// The id prefix (e.g. `credential`).
      pub const PREFIX: &'static str = $prefix;

      /// Generate a fresh, time-ordered id.
      pub fn generate() -> Self {
        Self($crate::mint($prefix))
      }

      /// Borrow the underlying string.
      pub fn as_str(&self) -> &str {
        &self.0
      }

      /// Consume into the underlying string.
      pub fn into_string(self) -> String {
        self.0
      }

      /// Wrap a string already known to be valid (e.g. one we minted and are
      /// round-tripping). Skips validation — use [`std::str::FromStr`] for
      /// untrusted input.
      pub fn from_trusted(s: impl Into<String>) -> Self {
        Self(s.into())
      }
    }

    impl ::std::str::FromStr for $name {
      type Err = $crate::ParseIdError;
      fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
        $crate::validate(s, $prefix)?;
        Ok(Self(s.to_string()))
      }
    }

    impl ::std::fmt::Display for $name {
      fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        f.write_str(&self.0)
      }
    }

    impl ::std::fmt::Debug for $name {
      fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(f, concat!(stringify!($name), "({})"), &self.0)
      }
    }
  };
}

// ─────────────────────────── Concrete ids ───────────────────────────

// Stored credentials (one TOML file per credential in the desktop app's
// credentials directory). Hidden from users; the effective primary identifier.
define_id!(pub struct CredentialId => "credential");

#[cfg(test)]
mod tests {
  use std::str::FromStr;

  use super::*;

  mod generate_tests {
    use super::*;

    #[test]
    fn generated_ids_carry_the_prefix() {
      let id = CredentialId::generate();
      assert!(id.as_str().starts_with("credential_"));
    }

    #[test]
    fn generated_ids_are_unique() {
      assert_ne!(CredentialId::generate(), CredentialId::generate());
    }

    #[test]
    fn generated_ids_validate() {
      let id = CredentialId::generate();
      assert!(CredentialId::from_str(id.as_str()).is_ok());
    }
  }

  mod parse_tests {
    use super::*;

    #[test]
    fn wrong_prefix_is_rejected() {
      let result = CredentialId::from_str("user_01j9abcdefghjkmnpqrstvwxyz");
      assert!(matches!(result, Err(ParseIdError::WrongPrefix { .. })));
    }

    #[test]
    fn short_entropy_is_rejected() {
      let result = CredentialId::from_str("credential_tooshort");
      assert!(matches!(result, Err(ParseIdError::BadUlid { .. })));
    }

    #[test]
    fn from_trusted_skips_validation() {
      let id = CredentialId::from_trusted("credential_legacy21charentropy00");
      assert_eq!(id.as_str(), "credential_legacy21charentropy00");
    }
  }

  mod serde_tests {
    use super::*;

    #[test]
    fn serializes_as_the_bare_string() {
      let id = CredentialId::from_trusted("credential_01j9abcdefghjkmnpqrstvwxyz");
      let json = serde_json::to_string(&id).unwrap();
      assert_eq!(json, "\"credential_01j9abcdefghjkmnpqrstvwxyz\"");
      let back: CredentialId = serde_json::from_str(&json).unwrap();
      assert_eq!(back, id);
    }
  }
}

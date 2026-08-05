//! Credential storage for third party services.
//!
//! Credentials live as individual TOML files in the app's credentials
//! directory (by default `~/Artcraft/artcraftx/credentials`). Users may
//! hand-write files there under any file name; the app periodically lists
//! and loads them all, and can rewrite managed files in place (e.g. to
//! refresh cookies or record success/failure timestamps).
//!
//! There are two layers:
//!
//! 1. [`credential_file::CredentialFile`] — the tolerant serialization
//!    layer. Everything optional so hand-written files parse.
//! 2. [`credential::Credential`] — the validated in-app layer. Exactly one
//!    secret (cookie XOR api key), tied back to its source file path.

pub mod api_key_credential;
pub mod artcraft_api_host;
pub mod cookie_credential;
pub mod credential;
pub mod credential_file;
pub mod find_service_credentials;
pub mod credential_user_info;
pub mod login_website;

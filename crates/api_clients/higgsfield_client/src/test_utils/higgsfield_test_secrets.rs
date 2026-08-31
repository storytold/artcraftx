//! Test-only loader for a browser-captured Higgsfield session kept off to
//! the side in the (gitignored) `external/credentials/higgsfield` directory.
//!
//! Files (all plain text, whitespace trimmed):
//!
//! - `cookies.txt` — the `cookie` header for higgsfield.ai, INCLUDING the
//!   `__client` cookie from `clerk.higgsfield.ai` (in DevTools: Application
//!   → Cookies → both hosts). This is the long-lived credential; the session
//!   mints bearer tokens from it.
//! - `bearer.txt` — optional. A captured `authorization: Bearer …` JWT. Only
//!   used when `cookies.txt` is absent (it expires after ~60s, so re-capture
//!   right before running).
//! - `datadome_client_id.txt` — optional. The `x-datadome-clientid` header.
//! - `job_id.txt` — optional. A job id for the status live tests.

use crate::credentials::higgsfield_auth::HiggsfieldAuth;
use crate::credentials::higgsfield_cookies::HiggsfieldCookies;
use crate::session::higgsfield_session::HiggsfieldSession;
use crate::types::ids::JobId;
use errors::AnyhowResult;
use std::fs::read_to_string;
use std::path::Path;

/// Repo-root-relative directory holding the captured secrets. Resolved from
/// this crate's manifest dir so tests work regardless of the shell's working
/// directory.
const SECRETS_DIR: &str = concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/../../../external/credentials/higgsfield",
);

/// The captured cookies.
pub fn load_higgsfield_test_cookies() -> AnyhowResult<HiggsfieldCookies> {
  Ok(HiggsfieldCookies::from_cookie_header(read_required("cookies.txt")?))
}

/// A session that mints its own tokens from the captured cookies.
pub fn load_higgsfield_test_session() -> AnyhowResult<HiggsfieldSession> {
  let mut session = HiggsfieldSession::from_cookies(load_higgsfield_test_cookies()?);
  if let Some(datadome_client_id) = read_optional("datadome_client_id.txt")? {
    session = session.with_datadome_client_id(datadome_client_id);
  }
  Ok(session)
}

/// A bearer auth for the raw endpoint bindings: minted from `cookies.txt`
/// when present, else the captured `bearer.txt`.
pub async fn load_higgsfield_test_auth() -> AnyhowResult<HiggsfieldAuth> {
  if read_optional("cookies.txt")?.is_some() {
    let session = load_higgsfield_test_session()?;
    return session.auth().await.map_err(|err| anyhow::anyhow!("Could not mint a Higgsfield session token: {err}"));
  }

  let bearer = read_required("bearer.txt")?;
  let mut auth = HiggsfieldAuth::new(bearer);
  if let Some(datadome_client_id) = read_optional("datadome_client_id.txt")? {
    auth = auth.with_datadome_client_id(datadome_client_id);
  }
  Ok(auth)
}

/// A job id to poll in the status live tests.
pub fn load_higgsfield_test_job_id() -> AnyhowResult<JobId> {
  Ok(JobId::new(read_required("job_id.txt")?))
}

fn read_required(file_name: &str) -> AnyhowResult<String> {
  read_optional(file_name)?.ok_or_else(|| {
    anyhow::anyhow!("Missing Higgsfield test secret {}/{} (capture it from a logged-in browser session)", SECRETS_DIR, file_name)
  })
}

fn read_optional(file_name: &str) -> AnyhowResult<Option<String>> {
  let path = Path::new(SECRETS_DIR).join(file_name);
  if !path.exists() {
    return Ok(None);
  }
  let raw = read_to_string(&path)
      .map_err(|err| anyhow::anyhow!("Could not read Higgsfield test secret {:?}: {}", path, err))?;
  let trimmed = raw.trim().to_string();
  Ok((!trimmed.is_empty()).then_some(trimmed))
}

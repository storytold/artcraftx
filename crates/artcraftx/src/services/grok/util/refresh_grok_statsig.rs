//! Preemptive Grok statsig refresh.
//!
//! When a Grok credential exists but its statsig material is missing or stale,
//! open a *hidden* grok.com webview so the passive capture harness records a
//! fresh signature the page makes on load, then persist the pieces onto the
//! credential. No-op when there's no credential or the material is still fresh.
//!
//! Relies on the shared WebKit session (the cookies the login window captured),
//! so the hidden window loads logged-in without re-authenticating.
//!
//! Concurrency: a process-wide single-flight guard ensures only one refresh
//! runs at a time — the credentials page mounts can fire the command twice (and
//! two concurrent same-label window builds deadlock the UI). The window is built
//! in the caller's async context (mirroring the working login flow) and only the
//! poll loop is spawned. Entirely best-effort: failures are logged, not
//! propagated.

use crate::credentials::auth_credential::CredentialSecret;
use crate::credentials::cookie_credential_grok_extra_pieces::{grok_statsig_needs_refresh, CookieCredentialGrokExtraPieces};
use crate::credentials::find_service_credentials::find_first_credential_for_service;
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::windows::login_window::utils::grok_statsig_capture::{grok_statsig_init_script, read_captured_statsig};
use chrono::Utc;
use core_types::enums::generation_source::GenerationSource;
use errors::{anyhow, AnyhowResult};
use grok_consumer_statsig::StatsigMaterial;
use log::{info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

const REFRESH_WINDOW_NAME: &str = "grok_statsig_refresh";
const GROK_URL: &str = "https://grok.com/";

/// Match the login-flow refresh window (see `add_web_credential_command`).
const REFRESH_MINUTES: i64 = 30;

/// Let the page load before reading cookies, then poll gently up to a timeout.
const INITIAL_LOAD_DELAY: Duration = Duration::from_secs(3);
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(20);
const CAPTURE_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Single-flight guard: only one refresh (and thus one refresh window) at a time.
static REFRESH_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Resets the single-flight flag on drop, so a panic or early return can never
/// wedge future refreshes.
struct RefreshGuard;

impl RefreshGuard {
  /// Acquire the guard, or `None` if a refresh is already running.
  fn acquire() -> Option<Self> {
    REFRESH_IN_PROGRESS
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .ok()
        .map(|_| RefreshGuard)
  }
}

impl Drop for RefreshGuard {
  fn drop(&mut self) {
    REFRESH_IN_PROGRESS.store(false, Ordering::SeqCst);
  }
}

/// Refresh the Grok statsig material if missing/stale. Returns immediately after
/// kicking off a background poll; safe to call on login, credentials-page mount,
/// etc. Non-blocking and best-effort.
pub async fn refresh_grok_statsig_if_stale(app: &AppHandle, app_data_root: &AppDataRoot) {
  let Some(credential) = find_first_credential_for_service(app_data_root, GenerationSource::GrokCookies) else {
    return; // Not logged in — nothing to refresh.
  };
  let grok_data = credential.cookies().and_then(|cookie| cookie.grok_data.as_ref());
  if !grok_statsig_needs_refresh(grok_data, Utc::now()) {
    return; // Still fresh.
  }

  let Some(guard) = RefreshGuard::acquire() else {
    return; // A refresh is already running.
  };

  if let Err(err) = start_refresh(app, app_data_root, guard) {
    warn!("Grok statsig refresh could not start: {err:?}");
    // `guard` was moved into start_refresh only on success; on error it has
    // already been dropped, releasing the flag.
  }
}

/// Build the hidden window (in the caller's context) and spawn the poll loop.
/// Ownership of `guard` transfers to the spawned task so the flag stays held
/// until the whole refresh finishes.
fn start_refresh(app: &AppHandle, app_data_root: &AppDataRoot, guard: RefreshGuard) -> AnyhowResult<()> {
  if app.get_window(REFRESH_WINDOW_NAME).is_some() {
    return Err(anyhow!("refresh window already exists"));
  }

  WebviewWindowBuilder::new(app, REFRESH_WINDOW_NAME, WebviewUrl::External(GROK_URL.parse()?))
      .title("Refreshing Grok…")
      .visible(false)
      .initialization_script(&grok_statsig_init_script())
      .build()?;

  info!("Grok statsig missing/stale — refreshing in a hidden window.");

  let app = app.clone();
  let app_data_root = app_data_root.clone();
  tauri::async_runtime::spawn(async move {
    let _guard = guard; // released on task completion (or panic)
    poll_and_persist(&app, &app_data_root).await;
    if let Some(window) = app.get_webview_window(REFRESH_WINDOW_NAME) {
      let _ = window.close();
    }
  });

  Ok(())
}

/// Poll the capture cookie the harness stashes until it appears or we time out,
/// then persist the material.
async fn poll_and_persist(app: &AppHandle, app_data_root: &AppDataRoot) {
  tokio::time::sleep(INITIAL_LOAD_DELAY).await;

  let deadline = tokio::time::Instant::now() + CAPTURE_TIMEOUT;
  loop {
    if let Some(window) = app.get_webview_window(REFRESH_WINDOW_NAME) {
      if let Some(material) = read_captured_statsig(&window) {
        if let Err(err) = save_refreshed_material(app_data_root, material) {
          warn!("Grok statsig refresh: could not save material: {err:?}");
        } else {
          info!("Refreshed Grok statsig material.");
        }
        return;
      }
    } else {
      return; // Window closed out from under us.
    }

    if tokio::time::Instant::now() >= deadline {
      warn!("Grok statsig refresh: no signed request observed within {CAPTURE_TIMEOUT:?}.");
      return;
    }
    tokio::time::sleep(CAPTURE_POLL_INTERVAL).await;
  }
}

/// Persist fresh material onto the existing Grok credential, updating only
/// `grok_data` (cookies untouched).
fn save_refreshed_material(app_data_root: &AppDataRoot, material: StatsigMaterial) -> AnyhowResult<()> {
  let mut credential = find_first_credential_for_service(app_data_root, GenerationSource::GrokCookies)
      .ok_or_else(|| anyhow!("Grok credential vanished mid-refresh"))?;

  let CredentialSecret::Cookies(cookie) = &mut credential.secret else {
    return Err(anyhow!("Grok credential is not a cookie credential"));
  };
  cookie.grok_data = Some(CookieCredentialGrokExtraPieces::fresh(material, Utc::now(), REFRESH_MINUTES));

  credential.save()?;
  Ok(())
}

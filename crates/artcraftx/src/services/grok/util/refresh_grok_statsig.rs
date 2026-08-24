//! Preemptive Grok statsig refresh.
//!
//! When a Grok credential exists but its statsig material is missing or stale,
//! open a *hidden* grok.com webview so the passive capture harness records a
//! fresh signature the page makes on load, then persist the pieces onto the
//! credential. No-op when there's no credential or the material is still fresh.
//!
//! Relies on the shared WebKit session (the cookies the login window captured),
//! so the hidden window loads logged-in without re-authenticating. It is
//! entirely best-effort: any failure is logged, never propagated.

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
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

const REFRESH_WINDOW_NAME: &str = "grok_statsig_refresh";
const GROK_URL: &str = "https://grok.com/";

/// Match the login-flow refresh window (see `add_web_credential_command`).
const REFRESH_MINUTES: i64 = 30;

/// How long to wait for the page to emit a signed request we can capture.
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(20);
const CAPTURE_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Refresh the Grok statsig material if it's missing or stale. Non-blocking and
/// best-effort — safe to call on login, when opening the credentials page, etc.
pub async fn refresh_grok_statsig_if_stale(app: AppHandle, app_data_root: AppDataRoot) {
  let Some(credential) = find_first_credential_for_service(&app_data_root, GenerationSource::GrokCookies) else {
    return; // Not logged in — nothing to refresh.
  };

  let grok_data = credential.cookies().and_then(|cookie| cookie.grok_data.as_ref());
  if !grok_statsig_needs_refresh(grok_data, Utc::now()) {
    return; // Still fresh.
  }

  if app.get_window(REFRESH_WINDOW_NAME).is_some() {
    return; // A refresh is already running.
  }

  info!("Grok statsig missing/stale — refreshing in a hidden window.");
  if let Err(err) = run_refresh(&app, &app_data_root).await {
    warn!("Grok statsig refresh failed: {err:?}");
  }
}

async fn run_refresh(app: &AppHandle, app_data_root: &AppDataRoot) -> AnyhowResult<()> {
  let window = WebviewWindowBuilder::new(
    app,
    REFRESH_WINDOW_NAME,
    WebviewUrl::External(GROK_URL.parse()?),
  )
      .title("Refreshing Grok…")
      .visible(false)
      .initialization_script(&grok_statsig_init_script())
      .build()?;

  // Poll the capture cookie the harness stashes until it appears or we time out.
  let deadline = tokio::time::Instant::now() + CAPTURE_TIMEOUT;
  let material = loop {
    if let Some(material) = read_captured_statsig(&window) {
      break Some(material);
    }
    if tokio::time::Instant::now() >= deadline {
      break None;
    }
    tokio::time::sleep(CAPTURE_POLL_INTERVAL).await;
  };

  let _ = window.close();

  match material {
    Some(material) => {
      save_refreshed_material(app_data_root, material)?;
      info!("Refreshed Grok statsig material.");
    }
    None => warn!("Grok statsig refresh: no signed request observed within {CAPTURE_TIMEOUT:?}."),
  }
  Ok(())
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

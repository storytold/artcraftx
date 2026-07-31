use crate::commands::credentials::add_web_credential_command::{save_web_credential, WebCredentialSave};
use crate::credentials::login_website::LoginWebsite;
use crate::events::basic_sendable_event_trait::BasicSendableEvent;
use crate::events::functional_events::refresh_account_state_event::RefreshAccountStateEvent;
use crate::login_window::extract_login_window_cookies::extract_login_window_cookies;
use crate::login_window::extract_user_info_from_cookies::extract_user_info_from_cookies;
use crate::login_window::login_window_trait::LoginWindowSite;
use crate::login_window::logins::login_site_for;
use crate::login_window::open_login_window::login_window_name;
use crate::state::data_dir::app_data_root::AppDataRoot;
use errors::AnyhowResult;
use log::{error, info};
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewWindow};

/// How often we poll the webview to see whether login has completed.
const POLL_INTERVAL: Duration = Duration::from_millis(2_000);

/// Watch a login webview until the user finishes signing in, then persist the
/// captured cookies (and any detected identity) and close the window.
///
/// Exits when the window is gone (user closed it) or once cookies are saved.
pub async fn login_window_thread(
  app: AppHandle,
  app_data_root: AppDataRoot,
  website: LoginWebsite,
) {
  let site = login_site_for(website);
  let window_name = login_window_name(website);
  let mut progressed_past_login = false;

  loop {
    let login_webview_window = match app.get_webview_window(&window_name) {
      Some(webview) => webview,
      None => {
        info!("Exit {} login thread (window gone).", website);
        return; // NB: Only exit if we don't have the webview.
      }
    };

    let result = check_login_window(
      &app,
      &login_webview_window,
      &app_data_root,
      website,
      site.as_ref(),
      &mut progressed_past_login,
    );

    match result {
      Err(err) => {
        error!("Error checking {} login window: {:?}", website, err);
      }
      Ok(false) => {} // Continue iteration and try again...
      Ok(true) => {
        info!("Saved {} cookies from login window. Closing.", website);
        if let Err(err) = login_webview_window.close() {
          error!("Error closing {} login window: {:?}", website, err);
        }
        return;
      }
    }

    tokio::time::sleep(POLL_INTERVAL).await;
  }
}

/// Returns `true` once cookies have been captured and saved (time to exit).
fn check_login_window(
  app_handle: &AppHandle,
  webview_window: &WebviewWindow,
  app_data_root: &AppDataRoot,
  website: LoginWebsite,
  site: &dyn LoginWindowSite,
  progressed_past_login: &mut bool,
) -> AnyhowResult<bool> {
  let url = webview_window.url()?;
  let hostname = url.host_str().unwrap_or_default().to_string();

  // On a third-party SSO / identity host: the user left the login page and is
  // mid auth-flow. Note the progress but don't capture yet.
  if site.auth_flow_hostnames().contains(&hostname.as_str()) {
    info!("{} webview is in auth flow; host `{}`.", website, hostname);
    *progressed_past_login = true;
    return Ok(false);
  }

  let at_destination = site.destination_hostnames().contains(&hostname.as_str());
  let on_login_page = at_destination && url.path() == site.login_url().path();

  // Reaching a non-login page on the destination host means the user got past
  // the login screen (e.g. same-domain email/password redirecting to the app).
  if at_destination && !on_login_page {
    *progressed_past_login = true;
  }

  if !(*progressed_past_login && at_destination) {
    return Ok(false);
  }

  let cookie_store = extract_login_window_cookies(webview_window, &site.cookie_urls())?;

  let session_cookie_names = site.session_cookie_names();
  let has_session_cookie = session_cookie_names.is_empty()
      || session_cookie_names.iter().any(|name| cookie_store.has_cookie(name));
  let has_enough_cookies = cookie_store.len() >= site.min_cookie_count();
  let cookie_length = cookie_store.calculate_approx_cookie_character_length();
  let has_big_enough_cookies = cookie_length >= site.min_cookie_char_length();

  info!(
    "{} login check: host `{}`, cookies={}, length={}, session_cookie={}",
    website, hostname, cookie_store.len(), cookie_length, has_session_cookie,
  );

  if !(has_session_cookie && has_enough_cookies && has_big_enough_cookies) {
    return Ok(false);
  }

  let maybe_user_info = extract_user_info_from_cookies(&cookie_store);
  if let Some(user_info) = &maybe_user_info {
    info!("Detected {} login identity: {:?}", website, user_info);
  }

  let credential = save_web_credential(
    app_data_root.credentials_dir(),
    WebCredentialSave {
      service: site.credential_service(),
      cookie_header: cookie_store.to_cookie_string(),
      maybe_user_info,
    },
  )?;

  info!("Saved {} credential to {:?}.", website, credential.source_path);

  let event = RefreshAccountStateEvent {
    provider: website.generation_provider(),
  };
  event.send_infallible(app_handle);

  Ok(true)
}

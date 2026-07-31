use crate::credentials::login_website::LoginWebsite;
use crate::login_window::login_window_thread::login_window_thread;
use crate::login_window::logins::login_site_for;
use crate::state::data_dir::app_data_root::AppDataRoot;
use crate::utils::clear_all_webview_cookies::clear_all_webview_cookies;
use anyhow::anyhow;
use errors::AnyhowResult;
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// Brief pause between navigations so each page settles (and to dodge
/// Cloudflare interstitials that trip on instant redirects).
const NAVIGATION_DELAY: Duration = Duration::from_millis(100);

/// Open a fresh, cookie-cleared login window for a website and start watching
/// it. The flow drives referrer -> homepage -> login page, then a background
/// thread captures cookies once the user finishes. See
/// [`login_window_thread`].
pub async fn open_login_window(
  app: &AppHandle,
  app_data_root: &AppDataRoot,
  website: LoginWebsite,
) -> AnyhowResult<()> {
  let window_name = login_window_name(website);
  if app.get_window(&window_name).is_some() {
    return Err(anyhow!("{} login window already open", website));
  }

  let site = login_site_for(website);

  let start_url = WebviewUrl::External(site.referring_url());
  let window = WebviewWindowBuilder::new(app, &window_name, start_url)
      .title(site.window_title())
      .center()
      .resizable(true)
      .visible(true)
      .closable(true)
      .min_inner_size(200.0, 800.0)
      .focused(true)
      .devtools(true)
      .build()?;

  let webview = window.get_webview(&window_name)
      .ok_or_else(|| anyhow!("no webview found"))?;

  // Start every login from a clean slate — no stale cookies/state.
  clear_all_webview_cookies(&webview)?;

  webview.navigate(site.opening_url())?;
  tokio::time::sleep(NAVIGATION_DELAY).await;
  webview.navigate(site.login_url())?;

  let app_handle = app.clone();
  let app_data_root = app_data_root.clone();

  let _ = tauri::async_runtime::spawn(async move {
    login_window_thread(app_handle, app_data_root, website).await;
  });

  Ok(())
}

/// The unique Tauri window label for a website's login window.
pub fn login_window_name(website: LoginWebsite) -> String {
  format!("{}_login_window", website.to_str())
}

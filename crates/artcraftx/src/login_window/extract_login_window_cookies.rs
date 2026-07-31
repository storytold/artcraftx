use cookie_store::cookie_store::CookieStore;
use errors::AnyhowResult;
use reqwest::Url;
use tauri::WebviewWindow;

/// Collect all cookies visible to the given URLs into a single [`CookieStore`].
///
/// Later URLs win on name collisions, but in practice a login only reads from
/// one or two origins for the same service.
pub fn extract_login_window_cookies(
  webview: &WebviewWindow,
  cookie_urls: &[Url],
) -> AnyhowResult<CookieStore> {
  let mut cookie_store = CookieStore::empty();
  for url in cookie_urls {
    let cookies = webview.cookies_for_url(url.clone())?;
    for cookie in cookies.iter() {
      cookie_store.add_cookie_name_and_value(
        cookie.name().to_string(),
        cookie.value().to_string(),
      );
    }
  }
  Ok(cookie_store)
}

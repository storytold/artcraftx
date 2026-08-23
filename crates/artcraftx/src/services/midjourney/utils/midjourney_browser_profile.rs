use browser_emulation::browser_profile::BrowserProfile;

/// The browser profile used for ALL first-party Midjourney HTTP/websocket
/// traffic (submit, imagine, index page, image downloads, websocket upgrade).
///
/// Cloudflare's `cf_clearance` cookie is bound to the User-Agent of the browser
/// that solved the challenge — here, the app's login webview, which on macOS is
/// a WKWebView (Safari engine). So we emulate Safari to keep the fingerprint
/// consistent with the captured cookies.
///
/// NB: for `cf_clearance` to validate, the emulated UA must match the webview's
/// actual UA closely. If Cloudflare still challenges, the next step is to
/// capture the webview's exact `navigator.userAgent` at login and use it as a
/// `BrowserProfile::Custom` UA override.
pub fn midjourney_browser_profile() -> BrowserProfile {
  BrowserProfile::Safari18
}

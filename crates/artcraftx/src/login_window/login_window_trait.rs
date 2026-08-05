use core_types::enums::generation_source::GenerationSource;
use crate::credentials::login_website::LoginWebsite;
use once_cell::sync::Lazy;
use reqwest::Url;

/// Referring website visited first, before the homepage. Defaults to Google so
/// the login page sees a plausible referrer (some providers gate on this).
static DEFAULT_REFERRING_URL: Lazy<Url> = Lazy::new(|| {
  Url::parse("https://google.com").expect("URL should parse")
});

/// Third-party SSO / identity provider hosts. When the webview is on one of
/// these we know the user is mid auth-flow (and has therefore left the login
/// page), so we never mistake it for the logged-in destination.
const DEFAULT_AUTH_FLOW_HOSTNAMES: &[&str] = &[
  "accounts.google.com",
  "accounts.youtube.com",
  "accounts.x.ai",
  "auth.openai.com",
  "appleid.apple.com",
  "login.live.com",
  "www.facebook.com",
  "facebook.com",
  "github.com",
  "discord.com",
  "x.com",
  "twitter.com",
];

/// A website whose login flow the app can drive in an embedded webview.
///
/// Implementors describe the URLs to visit and how to recognize a completed
/// login. Sensible defaults are provided for the referrer, the SSO host list,
/// and the completion heuristics; a site only overrides what differs.
///
/// The flow is: [`referring_url`] -> [`opening_url`] -> [`login_url`], and the
/// monitor thread watches for [`destination_hostnames`] plus cookie signals to
/// decide when the user has finished.
pub trait LoginWindowSite: Send + Sync {
  /// Which website this drives (also selects the credential service).
  fn login_website(&self) -> LoginWebsite;

  /// Human-facing window title (e.g. "Login to Artcraft").
  fn window_title(&self) -> String;

  /// The homepage, opened after the referrer and before the login page.
  /// e.g. `https://getartcraft.com`.
  fn opening_url(&self) -> Url;

  /// The login page itself. e.g. `https://app.getartcraft.com/login`.
  fn login_url(&self) -> Url;

  /// Hosts that indicate the user has reached the logged-in destination.
  /// e.g. `["app.getartcraft.com"]`. Note the login page is often on one of
  /// these too, so this is combined with cookie heuristics — see
  /// [`crate::login_window::login_window_thread`].
  fn destination_hostnames(&self) -> &[&str];

  /// The credential service that a successful login is saved under.
  fn credential_service(&self) -> GenerationSource {
    self.login_website().credential_service()
  }

  /// Referring website visited first. Defaults to Google.
  fn referring_url(&self) -> Url {
    DEFAULT_REFERRING_URL.clone()
  }

  /// SSO / identity-provider hosts that mean "still logging in".
  fn auth_flow_hostnames(&self) -> &[&str] {
    DEFAULT_AUTH_FLOW_HOSTNAMES
  }

  /// URLs whose cookies we read once the user appears logged in. Defaults to
  /// the origins of the opening and login pages.
  fn cookie_urls(&self) -> Vec<Url> {
    let mut urls = vec![origin_url(&self.login_url())];
    let opening_origin = origin_url(&self.opening_url());
    if !urls.contains(&opening_origin) {
      urls.push(opening_origin);
    }
    urls
  }

  /// Cookie names that, if present, strongly indicate a live session. When
  /// non-empty, at least one must be present before we consider login done.
  /// Empty by default (we fall back to size/count heuristics).
  fn session_cookie_names(&self) -> &[&str] {
    &[]
  }

  /// Minimum number of cookies expected once logged in.
  fn min_cookie_count(&self) -> usize {
    3
  }

  /// Minimum approximate cookie-header length (chars) expected once logged in.
  /// Session/auth tokens are large, so a real login pushes well past a page's
  /// baseline analytics cookies.
  fn min_cookie_char_length(&self) -> usize {
    800
  }
}

/// The scheme+host+port origin of a URL, with the path/query stripped — the
/// form Tauri's `cookies_for_url` expects.
pub fn origin_url(url: &Url) -> Url {
  let mut origin = url.clone();
  origin.set_path("/");
  origin.set_query(None);
  origin.set_fragment(None);
  origin
}

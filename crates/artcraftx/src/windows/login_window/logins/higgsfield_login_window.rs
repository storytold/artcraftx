use crate::credentials::login_website::LoginWebsite;
use crate::windows::login_window::login_journey::LoginJourney;
use crate::windows::login_window::login_window_trait::LoginWindowSite;
use once_cell::sync::Lazy;
use reqwest::Url;

const DESTINATION_HOSTNAMES: &[&str] = &["higgsfield.ai", "www.higgsfield.ai"];

/// Clerk's session JWT cookie, set on higgsfield.ai only while signed in.
const CLERK_SESSION_COOKIE: &str = "__session";

static WEBSITE_ENTRY_URL: Lazy<Url> = Lazy::new(|| {
  Url::parse("https://higgsfield.ai/").expect("URL should parse")
});

pub struct HiggsfieldLoginWindow;

impl LoginWindowSite for HiggsfieldLoginWindow {
  fn login_website(&self) -> LoginWebsite {
    LoginWebsite::Higgsfield
  }

  fn window_title(&self) -> String {
    "Login to Higgsfield".to_string()
  }

  /// Higgsfield has no distinct login page — the entry page hosts the login.
  fn journey(&self) -> LoginJourney {
    LoginJourney::with_default_pre_navigation()
        .website_entry(WEBSITE_ENTRY_URL.clone())
  }

  fn destination_hostnames(&self) -> &[&str] {
    DESTINATION_HOSTNAMES
  }

  // Higgsfield's auth is Clerk. The logged-OUT homepage already sets several
  // large cookies (DataDome, analytics), so the size/count fallback fires
  // before the user can sign in. Clerk only writes the `__session` JWT cookie
  // on higgsfield.ai once a session is active, so that's the login signal.
  // (`__client` on clerk.higgsfield.ai exists even signed out, and
  // `__client_uat` is present as "0" when signed out — neither is usable.)
  fn session_cookie_names(&self) -> &[&str] {
    &[CLERK_SESSION_COOKIE]
  }
}

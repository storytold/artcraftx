use crate::credentials::login_website::LoginWebsite;
use crate::windows::login_window::login_journey::LoginJourney;
use crate::windows::login_window::login_window_trait::LoginWindowSite;
use once_cell::sync::Lazy;
use reqwest::Url;

const DESTINATION_HOSTNAMES: &[&str] = &["higgsfield.ai", "www.higgsfield.ai"];

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
}

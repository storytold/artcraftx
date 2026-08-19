use crate::credentials::login_website::LoginWebsite;
use crate::windows::login_window::login_journey::LoginJourney;
use crate::windows::login_window::login_window_trait::LoginWindowSite;
use once_cell::sync::Lazy;
use reqwest::Url;

const DESTINATION_HOSTNAMES: &[&str] = &["grok.com", "www.grok.com"];

static WEBSITE_ENTRY_URL: Lazy<Url> = Lazy::new(|| {
  Url::parse("https://grok.com/").expect("URL should parse")
});

// NB: accounts.x.ai performs an auth dance whose details are still being
// figured out. It's listed in the default auth-flow hostnames, so the monitor
// thread treats time spent there as "still logging in" and waits for the
// return to grok.com.
static LOGIN_PAGE_URL: Lazy<Url> = Lazy::new(|| {
  Url::parse("https://accounts.x.ai/account").expect("URL should parse")
});

pub struct XAiLoginWindow;

impl LoginWindowSite for XAiLoginWindow {
  fn login_website(&self) -> LoginWebsite {
    LoginWebsite::XAi
  }

  fn window_title(&self) -> String {
    "Login to xAI".to_string()
  }

  fn journey(&self) -> LoginJourney {
    LoginJourney::with_default_pre_navigation()
        .website_entry(WEBSITE_ENTRY_URL.clone())
        .login_page(LOGIN_PAGE_URL.clone())
  }

  fn destination_hostnames(&self) -> &[&str] {
    DESTINATION_HOSTNAMES
  }
}

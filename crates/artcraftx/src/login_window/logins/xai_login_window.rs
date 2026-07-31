use crate::credentials::login_website::LoginWebsite;
use crate::login_window::login_window_trait::LoginWindowSite;
use once_cell::sync::Lazy;
use reqwest::Url;

const DESTINATION_HOSTNAMES: &[&str] = &["grok.com", "www.grok.com"];

static OPENING_URL: Lazy<Url> = Lazy::new(|| {
  Url::parse("https://grok.com/").expect("URL should parse")
});

static LOGIN_URL: Lazy<Url> = Lazy::new(|| {
  Url::parse("https://accounts.x.ai/sign-in?redirect=grok-com").expect("URL should parse")
});

pub struct XAiLoginWindow;

impl LoginWindowSite for XAiLoginWindow {
  fn login_website(&self) -> LoginWebsite {
    LoginWebsite::XAi
  }

  fn window_title(&self) -> String {
    "Login to xAI".to_string()
  }

  fn opening_url(&self) -> Url {
    OPENING_URL.clone()
  }

  fn login_url(&self) -> Url {
    LOGIN_URL.clone()
  }

  fn destination_hostnames(&self) -> &[&str] {
    DESTINATION_HOSTNAMES
  }
}

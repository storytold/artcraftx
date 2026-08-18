use crate::credentials::login_website::LoginWebsite;
use crate::windows::login_window::login_window_trait::LoginWindowSite;
use once_cell::sync::Lazy;
use reqwest::Url;

const DESTINATION_HOSTNAMES: &[&str] = &["app.runwayml.com"];

static OPENING_URL: Lazy<Url> = Lazy::new(|| {
  Url::parse("https://runwayml.com/").expect("URL should parse")
});

static LOGIN_URL: Lazy<Url> = Lazy::new(|| {
  Url::parse("https://app.runwayml.com/login").expect("URL should parse")
});

pub struct RunwayLoginWindow;

impl LoginWindowSite for RunwayLoginWindow {
  fn login_website(&self) -> LoginWebsite {
    LoginWebsite::Runway
  }

  fn window_title(&self) -> String {
    "Login to Runway".to_string()
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

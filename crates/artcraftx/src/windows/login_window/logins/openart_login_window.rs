use crate::credentials::login_website::LoginWebsite;
use crate::windows::login_window::login_window_trait::LoginWindowSite;
use once_cell::sync::Lazy;
use reqwest::Url;

const DESTINATION_HOSTNAMES: &[&str] = &["openart.ai", "www.openart.ai"];

static OPENING_URL: Lazy<Url> = Lazy::new(|| {
  Url::parse("https://openart.ai/").expect("URL should parse")
});

static LOGIN_URL: Lazy<Url> = Lazy::new(|| {
  Url::parse("https://openart.ai/login").expect("URL should parse")
});

pub struct OpenArtLoginWindow;

impl LoginWindowSite for OpenArtLoginWindow {
  fn login_website(&self) -> LoginWebsite {
    LoginWebsite::OpenArt
  }

  fn window_title(&self) -> String {
    "Login to OpenArt".to_string()
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

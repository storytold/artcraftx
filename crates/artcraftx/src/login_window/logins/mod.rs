pub mod artcraft_login_window;
pub mod higgsfield_login_window;
pub mod magnific_login_window;
pub mod openart_login_window;
pub mod runway_login_window;
pub mod xai_login_window;

use crate::credentials::login_website::LoginWebsite;
use crate::login_window::login_window_trait::LoginWindowSite;
use crate::login_window::logins::artcraft_login_window::ArtCraftLoginWindow;
use crate::login_window::logins::higgsfield_login_window::HiggsfieldLoginWindow;
use crate::login_window::logins::magnific_login_window::MagnificLoginWindow;
use crate::login_window::logins::openart_login_window::OpenArtLoginWindow;
use crate::login_window::logins::runway_login_window::RunwayLoginWindow;
use crate::login_window::logins::xai_login_window::XAiLoginWindow;

/// Resolve the [`LoginWindowSite`] driver for a website.
pub fn login_site_for(website: LoginWebsite) -> Box<dyn LoginWindowSite> {
  match website {
    LoginWebsite::ArtCraft => Box::new(ArtCraftLoginWindow),
    LoginWebsite::OpenArt => Box::new(OpenArtLoginWindow),
    LoginWebsite::Higgsfield => Box::new(HiggsfieldLoginWindow),
    LoginWebsite::Runway => Box::new(RunwayLoginWindow),
    LoginWebsite::Magnific => Box::new(MagnificLoginWindow),
    LoginWebsite::XAi => Box::new(XAiLoginWindow),
  }
}

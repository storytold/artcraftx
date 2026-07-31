//! Cookie-based web login windows for third-party sites.
//!
//! The frontend triggers a login for a [`crate::credentials::login_website::LoginWebsite`]
//! (via `open_web_login_command`). This module opens a fresh, cookie-cleared
//! Tauri webview, drives it through referrer -> homepage -> login page
//! ([`open_login_window`]), and watches it in the background
//! ([`login_window_thread`]) until the user finishes. Captured cookies (plus
//! any username/email decoded from a cookie JWT) are saved to the credentials
//! directory as a cookie [`crate::credentials::credential::Credential`].
//!
//! Each supported site implements [`login_window_trait::LoginWindowSite`]
//! under [`logins`], describing its URLs and completion heuristics.

pub mod extract_login_window_cookies;
pub mod extract_user_info_from_cookies;
pub mod login_window_thread;
pub mod login_window_trait;
pub mod logins;
pub mod open_login_window;

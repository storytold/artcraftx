/// The first-party (cookie-session) Grok client used by the router.
///
/// Drives the user's own logged-in grok.com session directly via captured
/// cookies (distinct from the API-key `RouterGrokApiClient`). Image generation
/// runs over Grok's "imagine" websocket and needs only the cookies; video
/// generation additionally needs an `x-statsig-id` signature.
pub struct RouterGrokClient {
  pub(crate) cookie_header: String,

  /// A browser-minted `x-statsig-id` for `POST /rest/app-chat/conversations/new`
  /// (video only). `None` when unavailable — image generation ignores it.
  pub(crate) maybe_statsig: Option<String>,
}

impl RouterGrokClient {
  pub fn new(cookie_header: String, maybe_statsig: Option<String>) -> Self {
    Self { cookie_header, maybe_statsig }
  }
}

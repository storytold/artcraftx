//! Harvesting fresh `x-statsig-id` signatures from a real browser and caching
//! everything decodable about them in `statsig.toml`.
//!
//! - [`statsig_cache_file`] — the `statsig.toml` schema, the decoder, and an
//!   offline test that prints a signature and its cacheable pieces (tool 1).
//! - [`harvest_via_webview`] — the live WebView harvester behind the
//!   `webview-harvest` feature, plus the `harvest_statsig` binary (tool 2).

pub mod statsig_cache_file;

#[cfg(feature = "webview-harvest")]
pub mod harvest_via_webview;

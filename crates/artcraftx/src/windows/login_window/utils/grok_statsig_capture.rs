//! Passive `x-statsig-id` capture during the Grok login.
//!
//! An initialization script installs `grok_consumer_statsig`'s observing
//! harness on the login page from first load, stashing each captured
//! `x-statsig-id` into a cookie — the only Rust-visible channel out of an
//! external webview in the login flow. After login the freshest capture is
//! decoded into storable pieces (seed, key, timestamp, …) via the library.
//!
//! This is deliberately passive: it records whatever signed request the page
//! makes on its own (no generation is triggered), so the stored statsig is a
//! session sample, not necessarily the video endpoint's.

use grok_consumer_statsig::statsig_cache_file::{decode_statsig, CapturedStatsig};
use grok_consumer_statsig::MINT_HARNESS_SCRIPT;
use tauri::WebviewWindow;

/// Cookie the capture harness stashes the latest statsig report into. Skipped
/// when persisting login cookies (see `extract_login_window_cookies`).
pub const STATSIG_CAPTURE_COOKIE: &str = "__artcraft_statsig_capture";

/// The Grok login webview's initialization script: define a cookie sink, then
/// install the observing harness. Runs before page scripts on every load, so it
/// catches the page's signed requests from the first one.
///
/// The harness reports `{method, path, statsigId, capturedAt}` JSON to
/// `window.__grokStatsigReport`; the sink flattens it into a cookie-safe
/// `method|path|statsigId|capturedAt` string (base64 statsig ids never contain
/// `|`), which [`read_captured_statsig`] reads back.
pub fn grok_statsig_init_script() -> String {
  format!(
    r#"(function () {{
  window.__grokStatsigReport = function (json) {{
    try {{
      var o = JSON.parse(json);
      var packed = [o.method, o.path, o.statsigId, o.capturedAt].join("|");
      document.cookie = "{cookie}=" + packed + "; path=/; max-age=3600";
    }} catch (e) {{ /* never break the page */ }}
  }};
}})();
{harness}"#,
    cookie = STATSIG_CAPTURE_COOKIE,
    harness = MINT_HARNESS_SCRIPT,
  )
}

/// Read and decode the freshest statsig the harness captured, if any.
pub fn read_captured_statsig(webview: &WebviewWindow) -> Option<CapturedStatsig> {
  let packed = webview
      .cookies()
      .ok()?
      .into_iter()
      .find(|cookie| cookie.name() == STATSIG_CAPTURE_COOKIE)
      .map(|cookie| cookie.value().to_string())?;

  parse_packed_statsig(&packed)
}

/// Parse the `method|path|statsigId|capturedAtMs` cookie payload and decode it.
fn parse_packed_statsig(packed: &str) -> Option<CapturedStatsig> {
  let mut parts = packed.splitn(4, '|');
  let method = parts.next()?;
  let path = parts.next()?;
  let statsig_id = parts.next()?;
  let captured_at_ms: i64 = parts.next()?.parse().ok()?;

  decode_statsig(statsig_id, method, path, captured_at_ms / 1000).ok()
}

#[cfg(test)]
mod tests {
  use super::*;

  // A real browser-captured statsig for POST /conversations/new.
  const CAPTURE: &str =
    "XbHvyYh7hmNc+8sVAGnrpcuHN/MK+mgN63cmnBljOJaoCKu6zTPVw6C7HVv1wculKCNmYVum+L1h0IjFmtyZD/C53M+ZXg";

  #[test]
  fn init_script_installs_sink_and_harness() {
    let script = grok_statsig_init_script();
    assert!(script.contains("__grokStatsigReport"));
    assert!(script.contains(STATSIG_CAPTURE_COOKIE));
    // The library harness is embedded.
    assert!(script.contains("x-statsig-id"));
  }

  #[test]
  fn parses_a_packed_capture_into_pieces() {
    let packed = format!("POST|/rest/app-chat/conversations/new|{CAPTURE}|1787535100000");
    let decoded = parse_packed_statsig(&packed).expect("should decode");
    assert_eq!(decoded.method, "POST");
    assert_eq!(decoded.path, "/rest/app-chat/conversations/new");
    assert_eq!(decoded.statsig_id, CAPTURE);
    assert_eq!(decoded.captured_at_unix, 1_787_535_100);
    assert_eq!(decoded.key, 93);
    assert!(!decoded.seed_b64.is_empty());
  }

  #[test]
  fn rejects_malformed_payloads() {
    assert!(parse_packed_statsig("garbage").is_none());
    assert!(parse_packed_statsig("POST|/p|not-base64!!|123").is_none());
  }
}

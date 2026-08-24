//! `x-statsig-id` request signing for grok.com's gated POST endpoints.
//!
//! grok.com gates its chat / media-generation POST endpoints (e.g.
//! `/rest/app-chat/conversations/new`) behind an `x-statsig-id` anti-bot
//! header. Around early 2026 Grok changed the signer, breaking the older
//! implementation that scraped a per-page-load verification token + SVG paths +
//! numbers from `index.html`.
//!
//! # Payload layout (verified) vs. live acceptance (currently NOT working)
//!
//! The 70-byte payload layout below is confirmed by decoding real captured
//! `x-statsig-id` values — XOR-masked by a random key byte, base64'd (standard
//! alphabet, no padding):
//! `[key][seed^key (48)][timestamp_LE^key (4)][sha256(msg)[..16]^key (16)][0x03^key]`,
//! where `msg = method!path!<number-decimal><salt><genuine-hex>` and
//! `number = unix - STATSIG_EPOCH`.
//!
//! The `genuine-hex` fingerprint is ported from the "pure" signer in
//! `aurora-develop/grok2api` (`internal/grok/statsig/pure.go` +
//! `svgfingerprint/compute.go`); [`svg_fingerprint`]'s self-consistency test
//! reproduces that repo's reference `(seed, hex)` vector byte-for-byte.
//!
//! **However, freshly-minted signatures are currently REJECTED by live grok.com
//! (403 "This page is out of date").** Decoding a *known-good* captured
//! `x-statsig-id` and running its own seed back through
//! [`svg_fingerprint::compute_hex_for_seed`] does **not** reproduce the
//! captured digest — so Grok's current signer computes `genuine-hex`
//! differently than this port. Inspecting the live `cdn.grok.com` bundles shows
//! why: the fingerprint's SVG path is no longer a fixed constant (it is built
//! dynamically from per-render segment data), and the signer samples the
//! actually-rendered, CSS-animated `loading-x-anim` SVG via `getComputedStyle`
//! + `requestAnimationFrame`. That is a real-DOM measurement a headless port
//! cannot reproduce without replicating the browser's animation/CSS engine and
//! the (rotating) path data. The literal salt is also absent from current
//! bundles, i.e. the signer has changed since the reference snapshot.
//!
//! ## Practical upshot
//!
//! - [`generate_statsig_id`] produces a *structurally* valid signature, but one
//!   Grok does not currently accept. The `generate_video` binding is otherwise
//!   proven correct end-to-end when supplied a **browser-captured** statsig via
//!   [`GrokRequestHeaders::statsig_id`] / [`PreCapturedStatsig`].
//! - For production, mint the statsig inside a real browser context — e.g. the
//!   app's own Tauri WebView loading grok.com — and read the generated header,
//!   rather than relying on this headless port.
//!
//! Build a signature with [`generate_statsig_id`] and attach it via
//! [`GrokRequestHeaders`], or use a [`LocalStatsigProvider`] / [`PreCapturedStatsig`].
//!
//! [`GrokRequestHeaders`]: crate::credentials::grok_request_headers::GrokRequestHeaders
//! [`GrokRequestHeaders::statsig_id`]: crate::credentials::grok_request_headers::GrokRequestHeaders

mod svg_fingerprint;

use crate::credentials::grok_request_headers::GrokRequestHeaders;
use crate::error::grok_error::GrokError;
use base64::prelude::BASE64_STANDARD_NO_PAD;
use base64::Engine;
use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Unix-time reference the signer counts from (2023-05-01).
pub(crate) const STATSIG_EPOCH: i64 = 1682924400;

/// Fixed salt folded into the signed message.
pub(crate) const STATSIG_SALT: &str = "obfiowerehiring";

/// Trailing marker byte of the assembled payload.
pub(crate) const STATSIG_MARK: u8 = 0x03;

/// Seed length in bytes.
pub(crate) const SEED_LEN: usize = 48;

/// A captured or computed `x-statsig-id` value.
#[derive(Clone, Debug)]
pub struct StatsigId(String);

impl StatsigId {
  pub fn new(value: impl Into<String>) -> Self {
    Self(value.into())
  }

  pub fn as_str(&self) -> &str {
    &self.0
  }

  pub fn into_string(self) -> String {
    self.0
  }

  /// Build [`GrokRequestHeaders`] carrying just this signature.
  pub fn into_request_headers(self) -> GrokRequestHeaders {
    GrokRequestHeaders {
      statsig_id: Some(self.0),
      ..Default::default()
    }
  }
}

/// Compute a fresh `x-statsig-id` for `method` + `path`
/// (e.g. `"POST"`, `"/rest/app-chat/conversations/new"`), using the current
/// time, a fresh random seed, and a random key byte.
pub fn generate_statsig_id(method: &str, path: &str) -> StatsigId {
  loop {
    let seed = random_seed();
    // A 48-byte seed always yields a fingerprint (see svg_fingerprint tests),
    // so this loops at most once; the retry is purely defensive.
    if let Some(hex) = svg_fingerprint::compute_hex_for_seed(&seed) {
      let key = random_byte();
      return build_statsig_id(method, path, Utc::now().timestamp(), &seed, &hex, key);
    }
  }
}

/// Deterministic core: assemble the signature from explicit inputs.
fn build_statsig_id(
  method: &str,
  path: &str,
  now_unix: i64,
  seed: &[u8],
  hex: &str,
  key: u8,
) -> StatsigId {
  let method = method.trim().to_ascii_uppercase();
  let path = path.trim();

  let number = now_unix.saturating_sub(STATSIG_EPOCH).max(0) as u32;

  // SHA-256 over method!path!<number-decimal><salt><seed-fingerprint-hex>.
  let input = format!("{method}!{path}!{number}{STATSIG_SALT}{hex}");
  let digest = Sha256::digest(input.as_bytes());

  // 70-byte payload: key, seed^key (48), number LE ^key (4), digest[..16]^key
  // (16), mark^key (1). Every region after byte 0 is XORed with the key.
  let number_bytes = number.to_le_bytes();
  let mut out = Vec::with_capacity(70);
  out.push(key);
  out.extend(seed.iter().map(|byte| byte ^ key));
  out.extend(number_bytes.iter().map(|byte| byte ^ key));
  out.extend(digest[..16].iter().map(|byte| byte ^ key));
  out.push(STATSIG_MARK ^ key);

  StatsigId(BASE64_STANDARD_NO_PAD.encode(out))
}

/// 48 random bytes (three v4 UUIDs); any bytes work, as the fingerprint is
/// derived from them.
fn random_seed() -> [u8; SEED_LEN] {
  let mut seed = [0u8; SEED_LEN];
  seed[0..16].copy_from_slice(&Uuid::new_v4().into_bytes());
  seed[16..32].copy_from_slice(&Uuid::new_v4().into_bytes());
  seed[32..48].copy_from_slice(&Uuid::new_v4().into_bytes());
  seed
}

fn random_byte() -> u8 {
  Uuid::new_v4().into_bytes()[0]
}

/// Supplies a fresh `x-statsig-id` for a specific request.
pub trait StatsigProvider {
  fn statsig_id(
    &self,
    method: &str,
    path: &str,
  ) -> impl std::future::Future<Output = Result<StatsigId, GrokError>> + Send;
}

/// Computes signatures locally with [`generate_statsig_id`].
#[derive(Clone, Copy, Debug, Default)]
pub struct LocalStatsigProvider;

impl StatsigProvider for LocalStatsigProvider {
  async fn statsig_id(&self, method: &str, path: &str) -> Result<StatsigId, GrokError> {
    Ok(generate_statsig_id(method, path))
  }
}

/// A [`StatsigProvider`] backed by a single pre-captured signature. Returns the
/// same value regardless of method/path.
#[derive(Clone, Debug)]
pub struct PreCapturedStatsig(pub StatsigId);

impl StatsigProvider for PreCapturedStatsig {
  async fn statsig_id(&self, _method: &str, _path: &str) -> Result<StatsigId, GrokError> {
    Ok(self.0.clone())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const PATH: &str = "/rest/app-chat/conversations/new";
  const NOW: i64 = 1761646073;

  fn seed_of(byte: u8) -> [u8; SEED_LEN] {
    [byte; SEED_LEN]
  }

  #[test]
  fn signature_decodes_to_70_bytes() {
    let decoded = BASE64_STANDARD_NO_PAD
        .decode(generate_statsig_id("POST", PATH).as_str())
        .unwrap();
    assert_eq!(decoded.len(), 70);
  }

  #[test]
  fn signature_is_94_chars() {
    // 70 bytes, no padding -> 94 chars, matching real captures.
    assert_eq!(generate_statsig_id("POST", PATH).as_str().len(), 94);
  }

  // Validate the byte layout with a fixed seed/key (key = 0 makes every XOR the
  // identity, exposing the raw regions).
  #[test]
  fn byte_layout_matches_the_algorithm() {
    let seed = seed_of(9);
    let hex = svg_fingerprint::compute_hex_for_seed(&seed).unwrap();
    let out = BASE64_STANDARD_NO_PAD
        .decode(build_statsig_id("POST", PATH, NOW, &seed, &hex, 0).as_str())
        .unwrap();
    assert_eq!(out.len(), 70);

    assert_eq!(out[0], 0, "byte 0 is the key");
    assert_eq!(&out[1..49], &seed, "bytes 1..49 are the seed");

    let number = (NOW - STATSIG_EPOCH) as u32;
    assert_eq!(&out[49..53], &number.to_le_bytes(), "bytes 49..53 are the LE timestamp");

    let input = format!("POST!{PATH}!{number}{STATSIG_SALT}{hex}");
    let digest = Sha256::digest(input.as_bytes());
    assert_eq!(&out[53..69], &digest[..16], "bytes 53..69 are the digest prefix");

    assert_eq!(out[69], STATSIG_MARK, "byte 69 is the mark");
  }

  #[test]
  fn key_xors_every_region_after_byte_zero() {
    let seed = seed_of(9);
    let hex = svg_fingerprint::compute_hex_for_seed(&seed).unwrap();
    let key = 0xA7u8;
    let out = BASE64_STANDARD_NO_PAD
        .decode(build_statsig_id("POST", PATH, NOW, &seed, &hex, key).as_str())
        .unwrap();
    let plain = BASE64_STANDARD_NO_PAD
        .decode(build_statsig_id("POST", PATH, NOW, &seed, &hex, 0).as_str())
        .unwrap();

    assert_eq!(out[0], key);
    for i in 1..70 {
      assert_eq!(out[i], plain[i] ^ key, "byte {i} must be plaintext XOR key");
    }
  }

  #[test]
  fn embedded_hex_folds_into_the_digest() {
    // A different seed -> different fingerprint -> different digest -> different
    // signature (even at the same time/key), proving the fingerprint is used.
    let a = {
      let seed = seed_of(1);
      let hex = svg_fingerprint::compute_hex_for_seed(&seed).unwrap();
      build_statsig_id("POST", PATH, NOW, &seed, &hex, 0)
    };
    let b = {
      let seed = seed_of(2);
      let hex = svg_fingerprint::compute_hex_for_seed(&seed).unwrap();
      build_statsig_id("POST", PATH, NOW, &seed, &hex, 0)
    };
    assert_ne!(a.as_str(), b.as_str());
  }

  #[test]
  fn into_request_headers_sets_only_statsig() {
    let headers = StatsigId::new("sig-abc").into_request_headers();
    assert_eq!(headers.statsig_id.as_deref(), Some("sig-abc"));
    assert!(headers.xai_request_id.is_none());
  }

  #[tokio::test]
  async fn local_provider_computes_a_valid_signature() {
    let statsig = LocalStatsigProvider.statsig_id("POST", PATH).await.unwrap();
    assert_eq!(statsig.as_str().len(), 94);
  }

}

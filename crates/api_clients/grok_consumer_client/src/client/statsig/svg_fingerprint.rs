//! The "genuine HEX" fingerprint that Grok's statsig signer derives from a
//! seed. Ported from `aurora-develop/grok2api`
//! (`internal/grok/statsig/svgfingerprint/compute.go`).
//!
//! Grok validates a statsig by recomputing this HEX from the seed embedded in
//! the payload, so any 48-byte seed works as long as we fold in the matching
//! HEX. It replays, in plain arithmetic, what the site's obfuscated JS derives
//! from a hidden SVG `<path>` + Web-Animations sample: pick a path by
//! `seed[5] % 4`, pick a cubic-bezier segment by `seed[5] % 16`, sample the
//! animation at a seed-derived time, and hex-encode the resulting RGBA +
//! rotation-matrix values the way JavaScript's `Number.toString(16)` would.

use std::f64::consts::PI;

/// The animation duration the `progress` is sampled against.
const ANIMATION_DURATION: f64 = 4096.0;

/// Compute the genuine HEX for a seed, or `None` if the seed can't drive the
/// fingerprint (too short, or a degenerate SVG segment — never happens for the
/// embedded paths with a full 48-byte seed).
pub fn compute_hex_for_seed(seed: &[u8]) -> Option<String> {
  if seed.len() < 25 {
    return None;
  }

  let path = DEFAULT_SVG_PATHS[seed[5] as usize % DEFAULT_SVG_PATHS.len()];
  let segments = path_number_segments(path);

  let seg_index = seed[5] as usize % 16;
  let segment = segments.get(seg_index)?;
  if segment.len() < 11 {
    return None;
  }

  let start_color = [segment[0], segment[1], segment[2]];
  let end_color = [segment[3], segment[4], segment[5]];
  let end_angle = scale_value(segment[6], 60.0, 360.0, true);
  let x1 = scale_value(segment[7], 0.0, 1.0, false);
  let y1 = scale_value(segment[8], -1.0, 1.0, false);
  let x2 = scale_value(segment[9], 0.0, 1.0, false);
  let y2 = scale_value(segment[10], -1.0, 1.0, false);

  let seek = (((seed[24] % 16) as u64 * (seed[22] % 16) as u64 * (seed[23] % 16) as u64) as f64
      / 10.0).round() * 10.0;
  let progress = cubic_bezier_y(x1, y1, x2, y2, seek / ANIMATION_DURATION);

  let r = css_color_channel(start_color[0], end_color[0], progress);
  let g = css_color_channel(start_color[1], end_color[1], progress);
  let b = css_color_channel(start_color[2], end_color[2], progress);
  let angle = end_angle * progress * PI / 180.0;
  let cos_v = angle.cos();
  let sin_v = angle.sin();

  let values = [r, g, b, cos_v, sin_v, -sin_v, cos_v, 0.0, 0.0];
  let mut buf = String::new();
  for value in values {
    buf.push_str(&number_to_hex(js_to_fixed(value, 2)));
  }
  Some(sanitize_hex(&buf))
}

/// Split the path (after its leading `M x,y C`) on `C` and pull the integer
/// runs out of each segment.
fn path_number_segments(path: &str) -> Vec<Vec<f64>> {
  if path.len() <= 9 {
    return Vec::new();
  }
  path[9..]
      .split('C')
      .map(extract_numbers)
      .filter(|nums| !nums.is_empty())
      .collect()
}

/// Every maximal run of ASCII digits, as f64. `.`/`-`/letters/commas/spaces all
/// act as separators (the embedded paths only contain positive integers).
fn extract_numbers(segment: &str) -> Vec<f64> {
  segment
      .split(|c: char| !c.is_ascii_digit())
      .filter(|run| !run.is_empty())
      .filter_map(|run| run.parse::<f64>().ok())
      .collect()
}

/// Map a 0..255 byte value into `[min, max]`, flooring or rounding to 2 dp.
fn scale_value(n: f64, min: f64, max: f64, floor: bool) -> f64 {
  let v = n * ((max - min) / 255.0) + min;
  if floor {
    v.floor()
  } else {
    js_to_fixed(v, 2)
  }
}

/// Interpolate a color channel, rounded and clamped to `[0, 255]`.
fn css_color_channel(start: f64, end: f64, progress: f64) -> f64 {
  let v = (start + (end - start) * progress).round();
  v.clamp(0.0, 255.0)
}

/// Sample the cubic-bezier easing `y` at `x` (Newton's method, then bisection),
/// matching the browser's `cubic-bezier(x1, y1, x2, y2)` timing function.
fn cubic_bezier_y(x1: f64, y1: f64, x2: f64, y2: f64, x: f64) -> f64 {
  if x <= 0.0 {
    return 0.0;
  }
  if x >= 1.0 {
    return 1.0;
  }

  let mut t = x;
  for _ in 0..8 {
    let x_at_t = sample_cubic(t, x1, x2) - x;
    if x_at_t.abs() < 1e-7 {
      return sample_cubic(t, y1, y2);
    }
    let d = sample_cubic_derivative(t, x1, x2);
    if d.abs() < 1e-7 {
      break;
    }
    t -= x_at_t / d;
  }

  let (mut lo, mut hi) = (0.0f64, 1.0f64);
  t = x;
  while lo < hi {
    let x_at_t = sample_cubic(t, x1, x2);
    if (x_at_t - x).abs() < 1e-7 {
      return sample_cubic(t, y1, y2);
    }
    if x > x_at_t {
      lo = t;
    } else {
      hi = t;
    }
    let next = (hi + lo) / 2.0;
    if next == t {
      break;
    }
    t = next;
  }
  sample_cubic(t, y1, y2)
}

fn sample_cubic(t: f64, a1: f64, a2: f64) -> f64 {
  ((1.0 - 3.0 * a2 + 3.0 * a1) * t + (3.0 * a2 - 6.0 * a1)) * t * t + 3.0 * a1 * t
}

fn sample_cubic_derivative(t: f64, a1: f64, a2: f64) -> f64 {
  (3.0 * (1.0 - 3.0 * a2 + 3.0 * a1) * t + 2.0 * (3.0 * a2 - 6.0 * a1)) * t + 3.0 * a1
}

/// JavaScript `Number(v.toFixed(precision))`: round half-away-from-zero to
/// `precision` decimals.
fn js_to_fixed(v: f64, precision: i32) -> f64 {
  let pow = 10f64.powi(precision);
  (v * pow).round() / pow
}

/// JavaScript `Number.prototype.toString(16)`.
fn number_to_hex(n: f64) -> String {
  if n == 0.0 {
    return "0".to_string();
  }
  if n == n.trunc() && n.abs() < 9007199254740992.0 {
    let int = n as i64;
    return if int < 0 { format!("-{:x}", -int) } else { format!("{:x}", int) };
  }
  float_to_hex_js(n)
}

/// Hex expansion of a non-integer float, matching JS semantics (up to 14
/// fractional hex digits).
fn float_to_hex_js(n: f64) -> String {
  if n < 0.0 {
    return format!("-{}", float_to_hex_js(-n));
  }
  if n == 0.0 {
    return "0".to_string();
  }

  let int_part = n as u64;
  let mut out = format!("{:x}", int_part);

  let mut frac = n - int_part as f64;
  if frac > 0.0 {
    out.push('.');
    for _ in 0..14 {
      if frac <= 0.0 {
        break;
      }
      frac *= 16.0;
      let mut digit = frac as u64;
      if digit > 15 {
        digit = 15;
      }
      out.push_str(&format!("{:x}", digit));
      frac -= digit as f64;
    }
  }

  out
}

/// Drop `.` and `-`, leaving a bare hex string.
fn sanitize_hex(s: &str) -> String {
  s.chars().filter(|&c| c != '.' && c != '-').collect()
}

/// The four hidden SVG paths Grok's signer selects from (by `seed[5] % 4`).
const DEFAULT_SVG_PATHS: [&str; 4] = [
  "M 10,30 C 202,167 243,238 50,9 h 101 s 53,236 92,62 C 183,211 231,32 79,32 h 212 s 177,182 47,35 C 239,79 13,166 84,159 h 3 s 52,122 82,64 C 135,46 167,243 207,93 h 185 s 53,51 174,249 C 167,105 13,127 93,97 h 1 s 82,247 113,216 C 141,225 31,57 85,81 h 224 s 89,74 87,116 C 72,183 62,34 48,21 h 55 s 0,5 124,62 C 6,158 39,101 63,253 h 45 s 152,200 201,164 C 53,182 133,87 119,220 h 255 s 138,213 214,18 C 62,247 43,239 182,13 h 107 s 238,188 198,254 C 169,156 237,209 230,249 h 73 s 22,110 87,116 C 231,172 154,252 178,106 h 94 s 13,30 102,215 C 206,110 66,71 157,77 h 126 s 94,77 102,79 C 123,221 171,198 227,123 h 94 s 49,65 222,147 C 58,201 175,209 43,247 h 95 s 26,25 43,80 C 180,184 254,148 197,87 h 123 s 227,38 117,121",
  "M 10,30 C 92,107 91,107 142,29 h 68 s 233,240 13,201 C 19,199 166,96 63,57 h 116 s 91,60 199,167 C 230,86 249,188 55,149 h 118 s 143,140 162,123 C 66,147 47,218 150,219 h 11 s 145,98 109,188 C 85,21 94,98 30,50 h 108 s 236,209 212,112 C 13,159 86,94 144,108 h 158 s 72,153 197,58 C 183,106 50,213 101,55 h 55 s 226,12 55,210 C 51,20 118,72 246,96 h 202 s 101,226 25,12 C 67,72 185,208 125,5 h 126 s 232,180 168,186 C 130,183 245,29 129,147 h 78 s 170,177 94,171 C 221,218 78,109 249,20 h 112 s 67,193 57,67 C 39,10 185,85 67,185 h 48 s 41,212 26,130 C 230,197 75,102 224,72 h 253 s 198,95 26,233 C 212,229 210,89 221,10 h 106 s 179,235 187,171 C 27,188 63,70 111,192 h 129 s 255,219 70,128 C 253,76 97,104 163,163 h 148 s 100,85 83,62",
  "M 10,30 C 3,142 215,98 78,231 h 145 s 226,98 100,95 C 176,15 12,17 28,42 h 115 s 94,179 227,198 C 138,151 125,127 137,1 h 182 s 139,246 224,5 C 100,182 243,133 120,6 h 152 s 240,164 96,85 C 78,216 22,78 188,239 h 19 s 44,188 41,17 C 102,116 224,115 28,219 h 237 s 123,38 184,218 C 70,113 93,123 243,8 h 110 s 44,219 143,252 C 193,139 11,47 183,27 h 162 s 191,97 238,138 C 203,96 186,119 113,6 h 241 s 62,45 35,239 C 189,194 24,103 180,203 h 156 s 229,76 226,172 C 232,84 123,215 86,104 h 109 s 177,207 71,244 C 215,49 76,4 159,174 h 169 s 64,10 128,177 C 22,51 158,116 100,105 h 1 s 83,84 53,217 C 30,61 199,197 127,151 h 76 s 90,130 88,80 C 132,156 76,146 33,243 h 7 s 8,169 171,76 C 92,39 69,45 49,88 h 85 s 47,126 99,148",
  "M 10,30 C 178,193 89,90 151,230 h 210 s 244,77 131,241 C 102,209 131,165 195,30 h 25 s 85,9 63,36 C 238,9 143,122 31,41 h 2 s 186,229 51,90 C 18,55 158,218 95,251 h 248 s 123,109 230,184 C 122,131 1,68 238,208 h 71 s 14,163 83,225 C 253,129 180,244 38,128 h 59 s 180,236 186,196 C 97,224 77,112 185,101 h 65 s 166,74 122,75 C 154,48 234,123 189,73 h 22 s 73,182 240,221 C 182,117 85,49 70,210 h 224 s 48,77 129,228 C 95,211 107,7 38,16 h 121 s 197,246 38,251 C 59,122 179,174 253,240 h 8 s 105,118 112,109 C 176,43 53,77 35,212 h 206 s 234,125 154,48 C 142,249 25,47 131,193 h 0 s 250,142 226,5 C 232,212 169,164 59,165 h 180 s 65,4 169,37 C 72,23 178,141 222,243 h 91 s 98,9 24,246 C 141,146 48,50 204,11 h 232 s 80,11 207,95",
];

#[cfg(test)]
mod tests {
  use super::*;
  use base64::prelude::BASE64_STANDARD;
  use base64::Engine;

  // Self-consistency vector from the reference implementation: the embedded
  // (seed, hex) pair must satisfy hex == fingerprint(seed).
  const GENUINE_SEED_B64: &str = "t2ODAFY4ozXd0K2Y8MdI2XfxTDiJoakZPuoaKfcQn8VuasZMcKliyhA1pJ+o1oMf";
  const GENUINE_HEX: &str = "3bab9506b851eb851eb840e8f5c28f5c28f80e8f5c28f5c28f806b851eb851eb8400";

  #[test]
  fn genuine_seed_reproduces_genuine_hex() {
    let seed = BASE64_STANDARD.decode(GENUINE_SEED_B64).unwrap();
    assert_eq!(compute_hex_for_seed(&seed).as_deref(), Some(GENUINE_HEX));
  }

  #[test]
  fn every_seed5_selects_a_usable_segment() {
    // All four paths have 16 segments with >= 11 numbers, so any 48-byte seed
    // yields a HEX regardless of seed[5].
    for seed5 in 0u8..=255 {
      let mut seed = [7u8; 48];
      seed[5] = seed5;
      assert!(compute_hex_for_seed(&seed).is_some(), "seed[5]={seed5} produced no HEX");
    }
  }

  #[test]
  fn hex_is_lowercase_hex_only() {
    let hex = compute_hex_for_seed(&[3u8; 48]).unwrap();
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()), "unexpected chars in {hex}");
  }
}

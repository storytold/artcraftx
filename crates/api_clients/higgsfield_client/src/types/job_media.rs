use serde::{Deserialize, Serialize};
use crate::types::string_enum::string_enum;

string_enum! {
  /// What kind of file a job result is.
  JobMediaType {
    Image => "image",
    Video => "video",
  }
}

/// One rendition of a job's output.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct JobMedia {
  #[serde(rename = "type")]
  pub media_type: JobMediaType,

  /// A CDN URL. Observed hosts are CloudFront; treat as time-limited.
  pub url: String,
}

/// A completed job's outputs: the full-resolution file plus a preview.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct JobResults {
  /// The full-resolution output (`.png` for images).
  pub raw: JobMedia,

  /// A smaller preview. For images this is usually a `.webp`, but it can be
  /// the same file as `raw` (observed for GPT Image 2).
  pub min: JobMedia,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn results_parse() {
    let json = r#"{
      "raw": {"type": "image", "url": "https://cdn.example.com/user_x/hf_20260101_000000_job.png"},
      "min": {"type": "image", "url": "https://cdn.example.com/user_x/hf_20260101_000000_job_min.webp"}
    }"#;
    let results: JobResults = serde_json::from_str(json).unwrap();
    assert_eq!(results.raw.media_type, JobMediaType::Image);
    assert!(results.raw.url.ends_with(".png"));
    assert!(results.min.url.ends_with("_min.webp"));
  }
}

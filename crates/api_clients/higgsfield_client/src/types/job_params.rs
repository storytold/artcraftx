use crate::types::image_quality::ImageQuality;
use crate::types::image_aspect_ratio::ImageAspectRatio;
use crate::types::image_resolution::ImageResolution;
use crate::types::image_seed::ImageSeed;
use crate::types::thinking_level::ThinkingLevel;
use serde::Deserialize;
use serde_json::Value;

/// The server's normalized view of a job's parameters, as echoed on job sets
/// and job status responses.
///
/// Every pipeline has its own shape (nano banana has `input_images`, GPT
/// image has `medias` / `quality` / `model`, ...), so only the fields common
/// to image jobs are typed; everything else stays available in `extra`.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct JobParams {
  #[serde(default)]
  pub prompt: Option<String>,

  #[serde(default)]
  pub aspect_ratio: Option<ImageAspectRatio>,

  #[serde(default)]
  pub resolution: Option<ImageResolution>,

  /// The server-derived pixel size (can differ from what was requested).
  #[serde(default)]
  pub width: Option<u32>,

  #[serde(default)]
  pub height: Option<u32>,

  #[serde(default)]
  pub batch_size: Option<u32>,

  /// GPT Image (low/medium/high) and Seedream lite / 4.5 (basic/high/ultra).
  #[serde(default)]
  pub quality: Option<ImageQuality>,

  /// Seedream models only.
  #[serde(default)]
  pub seed: Option<ImageSeed>,

  /// Nano Banana 2 Lite only.
  #[serde(default)]
  pub thinking: Option<ThinkingLevel>,

  /// GPT Image only: the sub-model actually used (e.g. `videotape-alpha`).
  #[serde(default)]
  pub model: Option<String>,

  /// Everything not typed above, keyed by field name.
  #[serde(flatten)]
  pub extra: serde_json::Map<String, Value>,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn nano_banana_params_parse() {
    let json = r#"{"width":864,"height":1184,"aspect_ratio":"3:4","resolution":"1k","batch_size":1,"input_images":[],"input_image":null,"application":null,"surface":null,"prompt":"a dinosaur on a skateboard"}"#;
    let params: JobParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.prompt.as_deref(), Some("a dinosaur on a skateboard"));
    assert_eq!(params.aspect_ratio, Some(ImageAspectRatio::Portrait3x4));
    assert_eq!(params.resolution, Some(ImageResolution::OneK));
    assert_eq!((params.width, params.height), (Some(864), Some(1184)));
    assert_eq!(params.batch_size, Some(1));
    assert!(params.quality.is_none());
    assert!(params.extra.contains_key("input_images"));
  }

  #[test]
  fn seedream_lite_params_parse() {
    let json = r#"{"prompt":"a corgi on a bike","medias":[],"batch_size":1,"aspect_ratio":"3:4","width":1728,"height":2304,"quality":"basic","seed":12745,"reference_elements":[]}"#;
    let params: JobParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.quality, Some(ImageQuality::Basic));
    assert_eq!(params.seed, Some(ImageSeed::new(12745)));
    assert!(params.resolution.is_none());
  }

  #[test]
  fn nano_banana_2_lite_params_parse() {
    let json = r#"{"width":864,"height":1184,"aspect_ratio":"3:4","batch_size":1,"thinking":"MINIMAL","is_inpaint":false,"prompt":"a corgi on a bike","medias":[],"reference_elements":[]}"#;
    let params: JobParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.thinking, Some(ThinkingLevel::Minimal));
    assert_eq!(params.extra.get("is_inpaint"), Some(&Value::Bool(false)));
  }

  #[test]
  fn gpt_image_params_parse() {
    let json = r#"{"width":1152,"height":2048,"prompt":"a corgi on a bike","medias":[],"aspect_ratio":"9:16","quality":"high","resolution":"2k","model":"videotape-alpha","remove_bg":false,"reference_elements":[]}"#;
    let params: JobParams = serde_json::from_str(json).unwrap();
    assert_eq!(params.quality, Some(ImageQuality::High));
    assert_eq!(params.model.as_deref(), Some("videotape-alpha"));
    assert_eq!(params.extra.get("remove_bg"), Some(&Value::Bool(false)));
  }
}

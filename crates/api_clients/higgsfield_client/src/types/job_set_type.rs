use crate::types::string_enum::string_enum;

string_enum! {
  /// Which model/pipeline a job set ran. This is the `type` on a job set and
  /// `job_set_type` on job status responses.
  JobSetType {
    /// Nano Banana Pro (the web app's name); `nano-banana-2` on the wire.
    NanoBanana2 => "nano_banana_2",
    GptImage2 => "gpt_image_2",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wire_round_trip() {
    for variant in JobSetType::known_variants() {
      assert_eq!(&JobSetType::from_str_lossy(variant.as_str()), variant);
    }
    let parsed: JobSetType = serde_json::from_str("\"seedance_2\"").unwrap();
    assert_eq!(parsed, JobSetType::Other("seedance_2".to_string()));
  }
}

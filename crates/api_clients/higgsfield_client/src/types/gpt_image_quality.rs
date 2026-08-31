use crate::types::string_enum::string_enum;

string_enum! {
  /// Quality tier for GPT Image generations.
  GptImageQuality {
    Low => "low",
    Medium => "medium",
    High => "high",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn wire_round_trip() {
    for variant in GptImageQuality::known_variants() {
      assert_eq!(&GptImageQuality::from_str_lossy(variant.as_str()), variant);
    }
    assert_eq!(serde_json::to_string(&GptImageQuality::High).unwrap(), "\"high\"");
  }
}

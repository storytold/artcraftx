use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

/// A sound the app can play for an event.
///
/// The unit variants are the frontend's sound catalog (`SoundManager.ts`) and
/// serialize to its keys (`"done"`, `"spike_throw"`, ...), so the preferences
/// API hands the frontend the same strings it plays by. NEVER change existing
/// serialized values; only add new ones.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppSoundFile {
  // Menu choices
  Click,
  #[serde(rename = "scifi_menu_beep_1")]
  ScifiMenuBeep1,
  #[serde(rename = "scifi_menu_beep_2")]
  ScifiMenuBeep2,
  ScifiMenuSelect,
  // Immediate enqueue success
  Done,
  // Immediate failure
  ErrorChirp,
  SpikeThrow,
  GiantShellKick,
  Wrong,
  // Async success
  SpecialFlower,
  ExtraPower,
  // Async errors
  Crumble,
  Ghost,
  SpecialAlert,
  ScifiAlert,
  ScifiShrillAlert,
  // Menus
  Next,
  Select,
  ScifiMenuOpen,
  ScifiMenuClose,
  // Reward / celebration
  Correct,
  Flower,
  // Trash / delete
  Trash,
  // Misc
  AcceptChirp,
  #[serde(rename = "accept_normal_level_1")]
  AcceptNormalLevel1,
  #[serde(rename = "accept_normal_level_2")]
  AcceptNormalLevel2,
  #[serde(rename = "accept_normal_level_3")]
  AcceptNormalLevel3,
  DeclineChirp,
  DeclineNormal,

  /// A user-supplied `.wav` file.
  /// NB: Serializes as `{"custom_wav": "/path/to/file.wav"}` in JSON and an
  /// inline table in TOML.
  CustomWav(PathBuf),
}

/// Serde helper for `Option<AppSoundFile>` fields: `None` (silent) is written
/// as the string `"none"` — TOML has no null, and the frontend already uses
/// `"none"` for "None (Silent)". Reading accepts `"none"`, `null`, or a sound.
pub mod optional_sound {
  use super::AppSoundFile;
  use serde::{Deserialize, Deserializer, Serialize, Serializer};

  pub const SILENT: &str = "none";

  #[derive(Serialize, Deserialize)]
  #[serde(untagged)]
  enum SoundOrSilent {
    Sound(AppSoundFile),
    Silent(String),
  }

  pub fn serialize<S: Serializer>(value: &Option<AppSoundFile>, serializer: S) -> Result<S::Ok, S::Error> {
    match value {
      Some(sound) => sound.serialize(serializer),
      None => SILENT.serialize(serializer),
    }
  }

  pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<AppSoundFile>, D::Error> {
    match Option::<SoundOrSilent>::deserialize(deserializer)? {
      None => Ok(None),
      Some(SoundOrSilent::Sound(sound)) => Ok(Some(sound)),
      Some(SoundOrSilent::Silent(text)) if text == SILENT => Ok(None),
      Some(SoundOrSilent::Silent(text)) => Err(serde::de::Error::custom(format!("unknown sound `{text}`"))),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn catalog_variants_serialize_to_frontend_keys() {
    assert_eq!(serde_json::to_string(&AppSoundFile::Done).unwrap(), "\"done\"");
    assert_eq!(serde_json::to_string(&AppSoundFile::SpikeThrow).unwrap(), "\"spike_throw\"");
    assert_eq!(serde_json::to_string(&AppSoundFile::ScifiMenuBeep1).unwrap(), "\"scifi_menu_beep_1\"");
    assert_eq!(serde_json::from_str::<AppSoundFile>("\"crumble\"").unwrap(), AppSoundFile::Crumble);
  }

  #[test]
  fn optional_sound_writes_silent_as_none_string() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Holder {
      #[serde(with = "optional_sound")]
      sound: Option<AppSoundFile>,
    }

    let silent = Holder { sound: None };
    assert_eq!(serde_json::to_string(&silent).unwrap(), "{\"sound\":\"none\"}");
    assert_eq!(serde_json::from_str::<Holder>("{\"sound\":\"none\"}").unwrap(), silent);
    assert_eq!(serde_json::from_str::<Holder>("{\"sound\":null}").unwrap(), silent);
    assert_eq!(toml::from_str::<Holder>("sound = \"none\"").unwrap(), silent);

    let done = Holder { sound: Some(AppSoundFile::Done) };
    assert_eq!(toml::to_string(&done).unwrap(), "sound = \"done\"\n");
    assert_eq!(toml::from_str::<Holder>("sound = \"done\"").unwrap(), done);
    assert!(serde_json::from_str::<Holder>("{\"sound\":\"bogus\"}").is_err());
  }

  #[test]
  fn custom_wav_serializes_as_tagged_path() {
    let sound = AppSoundFile::CustomWav("/tmp/ding.wav".into());
    assert_eq!(serde_json::to_string(&sound).unwrap(), "{\"custom_wav\":\"/tmp/ding.wav\"}");
    assert_eq!(serde_json::from_str::<AppSoundFile>("{\"custom_wav\":\"/tmp/ding.wav\"}").unwrap(), sound);
  }
}

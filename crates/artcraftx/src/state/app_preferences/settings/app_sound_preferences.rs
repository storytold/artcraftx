use crate::state::app_preferences::settings::app_sound_file::{optional_sound, AppSoundFile};
use serde_derive::{Deserialize, Serialize};

/// Which sound (if any) plays for each app event.
///
/// `None` means silent for that event and is stored as `"none"` (see
/// [`optional_sound`]). Missing fields in an older preferences file fall back
/// to the defaults.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSoundPreferences {
  /// Master switch: play sounds on events at all.
  pub play_sounds: bool,

  /// A file was deleted.
  #[serde(with = "optional_sound")]
  pub delete_file: Option<AppSoundFile>,

  /// A generation was accepted by the provider (image enqueue can be async).
  #[serde(with = "optional_sound")]
  pub enqueue_success: Option<AppSoundFile>,

  /// A generation was rejected on enqueue.
  #[serde(with = "optional_sound")]
  pub enqueue_failure: Option<AppSoundFile>,

  /// A generation finished.
  #[serde(with = "optional_sound")]
  pub generation_success: Option<AppSoundFile>,

  /// A generation failed.
  #[serde(with = "optional_sound")]
  pub generation_failure: Option<AppSoundFile>,
}

impl Default for AppSoundPreferences {
  fn default() -> Self {
    Self {
      play_sounds: true,
      delete_file: Some(AppSoundFile::Trash),
      enqueue_success: Some(AppSoundFile::Done),
      enqueue_failure: Some(AppSoundFile::SpikeThrow),
      generation_success: Some(AppSoundFile::SpecialFlower),
      generation_failure: Some(AppSoundFile::Crumble),
    }
  }
}

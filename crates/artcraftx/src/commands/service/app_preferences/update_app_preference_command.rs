use crate::state::app_preferences::app_preferences::AppPreferences;
use crate::state::app_preferences::app_preferences_manager::AppPreferencesManager;
use crate::state::app_preferences::settings::app_sound_file::{optional_sound, AppSoundFile};
use crate::state::app_preferences::settings::preferred_download_directory::PreferredDownloadDirectory;
use crate::state::app_preferences::settings::preferred_download_filename::PreferredDownloadFilename;
use anyhow::anyhow;
use errors::AnyhowResult;
use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use tauri::State;

/// One preference change, typed per preference. The frontend sends
/// `{ "preference": "<name>", "value": <value> }`; a sound preference with a
/// missing, `null`, or `"none"` value means "silent".
#[derive(Deserialize, Debug)]
#[serde(tag = "preference", content = "value", rename_all = "snake_case")]
pub enum UpdateAppPreferencesRequest {
  PreferredDownloadDirectory(PreferredDownloadDirectory),
  PreferredDownloadFilename(PreferredDownloadFilename),
  PlaySounds(bool),
  DeleteFileSound(#[serde(with = "optional_sound")] Option<AppSoundFile>),
  EnqueueSuccessSound(#[serde(with = "optional_sound")] Option<AppSoundFile>),
  EnqueueFailureSound(#[serde(with = "optional_sound")] Option<AppSoundFile>),
  GenerationSuccessSound(#[serde(with = "optional_sound")] Option<AppSoundFile>),
  GenerationFailureSound(#[serde(with = "optional_sound")] Option<AppSoundFile>),
}

#[derive(Serialize)]
pub struct UpdateAppPreferencesResponse {
  pub success: bool,
}

#[tauri::command]
pub async fn update_app_preferences_command(
  request: UpdateAppPreferencesRequest,
  app_prefs: State<'_, AppPreferencesManager>,
) -> Result<UpdateAppPreferencesResponse, String> {
  info!("update_app_preferences_command called: {:?}", request);

  update_prefs(request, &app_prefs).map_err(|err| {
    error!("Error updating app preferences: {:?}", err);
    format!("Error updating app preferences: {:?}", err)
  })?;

  Ok(UpdateAppPreferencesResponse { success: true })
}

fn update_prefs(request: UpdateAppPreferencesRequest, app_prefs: &AppPreferencesManager) -> AnyhowResult<()> {
  validate(&request)?;
  app_prefs.update(|prefs| apply(request, prefs))?;
  Ok(())
}

fn validate(request: &UpdateAppPreferencesRequest) -> AnyhowResult<()> {
  match request {
    UpdateAppPreferencesRequest::PreferredDownloadFilename(PreferredDownloadFilename::Custom(format)) => {
      PreferredDownloadFilename::validate_custom_format(format)
          .map_err(|reason| anyhow!("Invalid filename format: {}", reason))
    }
    _ => Ok(()),
  }
}

fn apply(request: UpdateAppPreferencesRequest, prefs: &mut AppPreferences) {
  use UpdateAppPreferencesRequest::*;
  match request {
    PreferredDownloadDirectory(directory) => prefs.downloads.preferred_download_directory = directory,
    PreferredDownloadFilename(filename) => prefs.downloads.preferred_download_filename = filename,
    PlaySounds(enabled) => prefs.sounds.play_sounds = enabled,
    DeleteFileSound(sound) => prefs.sounds.delete_file = sound,
    EnqueueSuccessSound(sound) => prefs.sounds.enqueue_success = sound,
    EnqueueFailureSound(sound) => prefs.sounds.enqueue_failure = sound,
    GenerationSuccessSound(sound) => prefs.sounds.generation_success = sound,
    GenerationFailureSound(sound) => prefs.sounds.generation_failure = sound,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn decode(json: &str) -> UpdateAppPreferencesRequest {
    serde_json::from_str(json).unwrap()
  }

  /// Pin the wire format the frontend sends (`UpdateAppPreference.ts`).
  #[test]
  fn decodes_each_preference() {
    assert!(matches!(
      decode(r#"{"preference":"play_sounds","value":true}"#),
      UpdateAppPreferencesRequest::PlaySounds(true),
    ));
    assert!(matches!(
      decode(r#"{"preference":"preferred_download_filename","value":"artcraft_convention"}"#),
      UpdateAppPreferencesRequest::PreferredDownloadFilename(PreferredDownloadFilename::ArtcraftConvention),
    ));
    assert!(matches!(
      decode(r#"{"preference":"preferred_download_filename","value":{"custom_format":"{model}_{date}"}}"#),
      UpdateAppPreferencesRequest::PreferredDownloadFilename(PreferredDownloadFilename::Custom(_)),
    ));
    assert!(matches!(
      decode(r#"{"preference":"preferred_download_directory","value":{"custom":"/tmp"}}"#),
      UpdateAppPreferencesRequest::PreferredDownloadDirectory(PreferredDownloadDirectory::Custom(_)),
    ));
    assert!(matches!(
      decode(r#"{"preference":"preferred_download_directory","value":{"system":"downloads"}}"#),
      UpdateAppPreferencesRequest::PreferredDownloadDirectory(PreferredDownloadDirectory::System(_)),
    ));
    assert!(matches!(
      decode(r#"{"preference":"enqueue_success_sound","value":"done"}"#),
      UpdateAppPreferencesRequest::EnqueueSuccessSound(Some(AppSoundFile::Done)),
    ));
    assert!(matches!(
      decode(r#"{"preference":"generation_failure_sound","value":{"custom_wav":"/tmp/x.wav"}}"#),
      UpdateAppPreferencesRequest::GenerationFailureSound(Some(AppSoundFile::CustomWav(_))),
    ));
  }

  /// "None (Silent)" in the UI sends `value: undefined` (key absent) — and
  /// `null` must work too.
  #[test]
  fn silent_sound_decodes_from_missing_or_null_value() {
    assert!(matches!(
      decode(r#"{"preference":"delete_file_sound"}"#),
      UpdateAppPreferencesRequest::DeleteFileSound(None),
    ));
    assert!(matches!(
      decode(r#"{"preference":"delete_file_sound","value":null}"#),
      UpdateAppPreferencesRequest::DeleteFileSound(None),
    ));
    assert!(matches!(
      decode(r#"{"preference":"delete_file_sound","value":"none"}"#),
      UpdateAppPreferencesRequest::DeleteFileSound(None),
    ));
  }

  #[test]
  fn wrong_value_type_is_rejected() {
    assert!(serde_json::from_str::<UpdateAppPreferencesRequest>(r#"{"preference":"play_sounds","value":"yes"}"#).is_err());
    assert!(serde_json::from_str::<UpdateAppPreferencesRequest>(r#"{"preference":"enqueue_success_sound","value":"not_a_sound"}"#).is_err());
    assert!(serde_json::from_str::<UpdateAppPreferencesRequest>(r#"{"preference":"generation_enqueue_sound","value":"done"}"#).is_err());
  }

  #[test]
  fn custom_filename_format_is_validated() {
    let bad = decode(r#"{"preference":"preferred_download_filename","value":{"custom_format":"a/b"}}"#);
    assert!(validate(&bad).is_err());
    let good = decode(r#"{"preference":"preferred_download_filename","value":{"custom_format":"{model}_{date}"}}"#);
    assert!(validate(&good).is_ok());
  }

  #[test]
  fn apply_writes_into_the_right_group() {
    let mut prefs = AppPreferences::default();
    apply(decode(r#"{"preference":"play_sounds","value":false}"#), &mut prefs);
    apply(decode(r#"{"preference":"generation_success_sound"}"#), &mut prefs);
    apply(decode(r#"{"preference":"preferred_download_directory","value":{"custom":"/tmp/out"}}"#), &mut prefs);
    assert!(!prefs.sounds.play_sounds);
    assert_eq!(prefs.sounds.generation_success, None);
    assert_eq!(prefs.downloads.preferred_download_directory, PreferredDownloadDirectory::Custom("/tmp/out".into()));
  }
}

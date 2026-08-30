import { invoke } from "@tauri-apps/api/core";

export interface GetAppPreferencesResult {
  preferences: AppPreferencesPayload,
}

// Mirrors the backend's `AppPreferences` (grouped like the on-disk TOML).
export interface AppPreferencesPayload {
  sounds: AppSoundPreferences,
  downloads: AppDownloadPreferences,
}

export interface AppSoundPreferences {
  // Master switch: play sounds on events at all.
  play_sounds: boolean,

  // Which sound plays for each event. A catalog key (defined in the frontend
  // `SoundManager`), a custom .wav, or "none" for silent.
  delete_file: AppSoundFile,
  enqueue_success: AppSoundFile,
  enqueue_failure: AppSoundFile,
  generation_success: AppSoundFile,
  generation_failure: AppSoundFile,
}

// A sound catalog key (e.g. "done"), a custom .wav, or "none" (silent).
export type AppSoundFile = string | CustomWavSound;

export interface CustomWavSound {
  custom_wav: string,
}

export interface AppDownloadPreferences {
  // Preferred download directory
  preferred_download_directory: PreferredDownloadDirectory,

  // How downloaded generation files are named on disk.
  preferred_download_filename: PreferredDownloadFilename,
}

export type PreferredDownloadDirectory = SystemDirectory | CustomDirectory;

// "artcraft_convention" or a custom format string wrapper.
// Custom formats accept {model}, {date}, {YYYY}, {YY}, {MM}, {DD}, {HH},
// {mm}, {SS}, and {batch_index} tokens.
export type PreferredDownloadFilename = "artcraft_convention" | CustomFilenameFormat;

export interface CustomFilenameFormat {
  custom_format: string,
}

export interface SystemDirectory {
  // If the directory is a system directory.
  system: string,
}

export interface CustomDirectory {
  // If the directory is a custom user directory.
  custom: string,
}

export const GetAppPreferences = async () : Promise<GetAppPreferencesResult> => {
  let result = await invoke("get_app_preferences_command");
  return (result as GetAppPreferencesResult);
}

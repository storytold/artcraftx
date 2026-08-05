import { invoke } from "@tauri-apps/api/core";

export interface GetAppPreferencesResult {
  preferences: AppPreferencesPayload,
}

export interface AppPreferencesPayload {
  // Preferred download directory
  preferred_download_directory: PreferredDownloadDirectory,

  // How downloaded generation files are named on disk.
  preferred_download_filename: PreferredDownloadFilename,

  // Play sounds on events.
  play_sounds: boolean,

  // Sound names to play. 
  // These are keys, not filenames, and are defined by the frontend.
  delete_file_sound?: string,
  enqueue_success_sound?: string,
  enqueue_failure_sound?: string,
  generation_success_sound?: string,
  generation_failure_sound?: string,
  generation_enqueue_sound?: string,
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

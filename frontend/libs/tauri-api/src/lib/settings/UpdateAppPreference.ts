
import { invoke } from "@tauri-apps/api/core";
import { AppSoundFile, PreferredDownloadDirectory, PreferredDownloadFilename } from "./GetAppPreferences";

// One preference change. For the sound preferences, `undefined` or "none"
// means silent.
export interface UpdateAppPreferencesRequest {
  preference: PreferenceName,
  value: undefined | boolean | AppSoundFile | PreferredDownloadDirectory | PreferredDownloadFilename,
}

export enum PreferenceName {
  PreferredDownloadDirectory = "preferred_download_directory",
  PreferredDownloadFilename = "preferred_download_filename",
  PlaySounds = "play_sounds", 
  DeleteFileSound = "delete_file_sound",
  EnqueueSuccessSound = "enqueue_success_sound",
  EnqueueFailureSound = "enqueue_failure_sound",
  GenerationSuccessSound = "generation_success_sound",
  GenerationFailureSound = "generation_failure_sound",
}

export interface UpdateAppPreferencesResult {
  success: boolean
}

export const UpdateAppPreferences = async (request: UpdateAppPreferencesRequest) : Promise<UpdateAppPreferencesResult> => {
  let result = await invoke("update_app_preferences_command", { 
    request: {
      preference: request.preference,
      value: request.value,
    }
  });
  return (result as UpdateAppPreferencesResult);
}

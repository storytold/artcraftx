import { useEffect, useState } from "react";
import { SoundManager } from "@storyteller/soundboard";
import { Button } from "@storyteller/ui-button";
import { Play } from "lucide-react";
import {
  AppPreferencesPayload,
  AppSoundFile,
  GetAppPreferences,
} from "@storyteller/tauri-api";
import { PreferenceName, UpdateAppPreferences } from "@storyteller/tauri-api";
import { Select, SelectValue } from "@storyteller/ui-select";
import { Switch } from "@storyteller/ui-switch";
import { Label } from "@storyteller/ui-label";

interface AudioSettingsPaneProps {}

export const AudioSettingsPane = (args: AudioSettingsPaneProps) => {
  const [preferences, setPreferences] = useState<
    AppPreferencesPayload | undefined
  >(undefined);

  useEffect(() => {
    const fetchData = async () => {
      const prefs = await GetAppPreferences();
      setPreferences(prefs.preferences);
    };
    fetchData();
  }, []);

  const sounds = preferences?.sounds;
  const playSounds = sounds?.play_sounds || false;

  const deleteFileSound = orNone(sounds?.delete_file);
  const enqueueSuccessSound = orNone(sounds?.enqueue_success);
  const enqueueFailureSound = orNone(sounds?.enqueue_failure);
  const generationSuccessSound = orNone(sounds?.generation_success);
  const generationFailureSound = orNone(sounds?.generation_failure);

  const reloadPreferences = async () => {
    const prefs = await GetAppPreferences();
    setPreferences(prefs.preferences);
  };

  const setPlaySounds = async (checked: boolean) => {
    //const value = checked ? "true" : "false";
    await UpdateAppPreferences({
      preference: PreferenceName.PlaySounds,
      value: checked,
    });
    await reloadPreferences();
  };

  const setDeleteFileSound = async (val: string) => {
    let sendVal = val === "none" ? undefined : val;
    await UpdateAppPreferences({
      preference: PreferenceName.DeleteFileSound,
      value: sendVal,
    });
    SoundManager.playPreview(val);
    await reloadPreferences();
  };

  const setEnqueueSuccessSound = async (val: string) => {
    let sendVal = val === "none" ? undefined : val;
    await UpdateAppPreferences({
      preference: PreferenceName.EnqueueSuccessSound,
      value: sendVal,
    });
    SoundManager.playPreview(val);
    await reloadPreferences();
  };

  const setEnqueueFailureSound = async (val: string) => {
    let sendVal = val === "none" ? undefined : val;
    await UpdateAppPreferences({
      preference: PreferenceName.EnqueueFailureSound,
      value: sendVal,
    });
    SoundManager.playPreview(val);
    await reloadPreferences();
  };

  const setSuccessSound = async (val: string) => {
    let sendVal = val === "none" ? undefined : val;
    await UpdateAppPreferences({
      preference: PreferenceName.GenerationSuccessSound,
      value: sendVal,
    });
    SoundManager.playPreview(val);
    await reloadPreferences();
  };

  const setFailureSound = async (val: string) => {
    let sendVal = val === "none" ? undefined : val;
    await UpdateAppPreferences({
      preference: PreferenceName.GenerationFailureSound,
      value: sendVal,
    });
    SoundManager.playPreview(val);
    await reloadPreferences();
  };

  const playSound = (val?: string) => {
    if (val !== undefined && val !== "none") {
      SoundManager.playPreview(val);
    }
  };

  return (
    <>
      <div className="space-y-4">
        <div className="flex flex-col">
          <Label htmlFor="play-sounds">
            Play Notification Sounds for Events?
          </Label>
          <Switch enabled={playSounds} setEnabled={setPlaySounds} />
        </div>

        <div className="space-y-1">
          <Label htmlFor="success-sound">Delete File Sound</Label>
          <div className="flex items-center gap-2">
            <Select
              id="success-sound"
              value={deleteFileSound}
              onChange={(val: SelectValue) => setDeleteFileSound(val as string)}
              options={SoundManager.OPTIONS}
              className="grow"
            />
            <Button
              variant="primary"
              className="w-[40px] h-[40px]"
              icon={Play}
              onClick={() => playSound(deleteFileSound)}
            />
          </div>
        </div>

        <div className="space-y-1">
          <Label htmlFor="success-sound">Enqueue Success Sound</Label>
          <div className="flex items-center gap-2">
            <Select
              id="success-sound"
              value={enqueueSuccessSound}
              onChange={(val: SelectValue) => setEnqueueSuccessSound(val as string)}
              options={SoundManager.OPTIONS}
              className="grow"
            />
            <Button
              variant="primary"
              className="w-[40px] h-[40px]"
              icon={Play}
              onClick={() => playSound(enqueueSuccessSound)}
            />
          </div>
        </div>

        <div className="space-y-1">
          <Label htmlFor="success-sound">Enqueue Failure Sound</Label>
          <div className="flex items-center gap-2">
            <Select
              id="success-sound"
              value={enqueueFailureSound}
              onChange={(val: SelectValue) => setEnqueueFailureSound(val as string)}
              options={SoundManager.OPTIONS}
              className="grow"
            />
            <Button
              variant="primary"
              className="w-[40px] h-[40px]"
              icon={Play}
              onClick={() => playSound(enqueueFailureSound)}
            />
          </div>
        </div>

        <div className="space-y-1">
          <Label htmlFor="success-sound">Generation Success Sound</Label>
          <div className="flex items-center gap-2">
            <Select
              id="success-sound"
              value={generationSuccessSound}
              onChange={(val: SelectValue) => setSuccessSound(val as string)}
              options={SoundManager.OPTIONS}
              className="grow"
            />
            <Button
              variant="primary"
              className="w-[40px] h-[40px]"
              icon={Play}
              onClick={() => playSound(generationSuccessSound)}
            />
          </div>
        </div>

        <div className="space-y-1">
          <Label htmlFor="failure-sound">Generation Failure Sound</Label>
          <div className="flex items-center gap-2">
            <Select
              id="failure-sound"
              value={generationFailureSound}
              onChange={(val: SelectValue) => setFailureSound(val as string)}
              options={SoundManager.OPTIONS}
              className="grow"
            />
            <Button
              variant="primary"
              className="w-[40px] h-[40px]"
              icon={Play}
              onClick={() => playSound(generationFailureSound)}
            />
          </div>
        </div>

      </div>
    </>
  );
};

// The dropdown only knows catalog keys; anything else (silent, or a custom
// .wav the UI can't pick yet) shows as "None (Silent)".
const orNone = (val: AppSoundFile | undefined | null): string => {
  if (!val || typeof val !== "string") {
    return "none";
  }
  return val;
};

import { useEffect, useState } from "react";
import { Button } from "@storyteller/ui-button";
import {
  AppPreferencesPayload,
  CustomDirectory,
  GetAppPreferences,
  SystemDirectory,
} from "@storyteller/tauri-api";
import { PreferenceName, UpdateAppPreferences } from "@storyteller/tauri-api";
import { open } from "@tauri-apps/plugin-dialog";
import { Switch } from "@storyteller/ui-switch";
import { DownloadDirectoryReveal } from "@storyteller/tauri-api";
import { Folder, RotateCcw, Search } from "lucide-react";
import { useEnterToGenerateStore } from "@storyteller/ui-promptbox";
import { SettingsBlock, SettingsRow } from "./SettingsRow";
import {
  getAskLocationBeforeDownload,
  setAskLocationBeforeDownload,
} from "@storyteller/api";

interface MiscSettingsPaneProps {}

export const MiscSettingsPane = (args: MiscSettingsPaneProps) => {
  const [preferences, setPreferences] = useState<
    AppPreferencesPayload | undefined
  >(undefined);

  const enterToGenerate = useEnterToGenerateStore((s) => s.enabled);
  const setEnterToGenerate = useEnterToGenerateStore((s) => s.setEnabled);

  const [askLocationBeforeDownload, setAskLocationBeforeDownloadState] =
    useState<boolean>(() => getAskLocationBeforeDownload());

  const toggleAskLocationBeforeDownload = (enabled: boolean) => {
    setAskLocationBeforeDownload(enabled);
    setAskLocationBeforeDownloadState(enabled);
  };

  useEffect(() => {
    const fetchData = async () => {
      const prefs = await GetAppPreferences();
      console.log("prefs", prefs);
      setPreferences(prefs.preferences);
    };
    fetchData();
  }, []);

  // NB: This might be a complex type.
  const outerDownloadObject = preferences?.preferred_download_directory || {};
  const downloadDirectory =
    "custom" in outerDownloadObject
      ? (outerDownloadObject.custom as string)
      : "";
  const currentDownloadLabel =
    "system" in outerDownloadObject
      ? "System Download Directory"
      : downloadDirectory;

  const reloadPreferences = async () => {
    const prefs = await GetAppPreferences();
    console.log("prefs", prefs);
    setPreferences(prefs.preferences);
  };

  const openDirectoryPicker = async () => {
    let directory = await open({
      multiple: false,
      directory: true,
      defaultPath: downloadDirectory || undefined,
    });
    if (directory === null) {
      return; // User dismissed the dialog choice
    }
    await UpdateAppPreferences({
      preference: PreferenceName.PreferredDownloadDirectory,
      value: {
        custom: directory,
      } as CustomDirectory,
    });
    await reloadPreferences();
  };

  const clearDirectory = async () => {
    await UpdateAppPreferences({
      preference: PreferenceName.PreferredDownloadDirectory,
      value: {
        system: "downloads",
      } as SystemDirectory,
    });
    await reloadPreferences();
  };

  const showDirectory = async () => {
    await DownloadDirectoryReveal();
  };

  return (
    <div className="text-base-fg">
      <SettingsBlock
        title="Download directory"
        description="Generated files are written straight to this folder."
      >
        <div className="w-full overflow-x-auto rounded-ax-sm border border-line bg-well/60 px-3 py-2 font-mono text-[12px] text-putty">
          {currentDownloadLabel}
        </div>
        <div className="flex flex-wrap gap-2">
          <Button variant="secondary" onClick={openDirectoryPicker}>
            <Folder className="h-3.5 w-3.5" />
            Choose folder
          </Button>
          <Button variant="secondary" onClick={showDirectory}>
            <Search className="h-3.5 w-3.5" />
            Show in explorer
          </Button>
          <Button variant="ghost" onClick={clearDirectory}>
            <RotateCcw className="h-3.5 w-3.5" />
            Use default
          </Button>
        </div>
      </SettingsBlock>
      <SettingsRow
        title="Ask location before download"
        description="When on, a file picker appears for every download so you choose where each file goes. When off, downloads go straight to the folder above."
      >
        <Switch
          enabled={askLocationBeforeDownload}
          setEnabled={toggleAskLocationBeforeDownload}
        />
      </SettingsRow>
      <SettingsRow
        title="Enter to generate"
        description="When on, Enter submits the prompt and Shift+Enter adds a new line. When off, both add a new line — use the button to submit."
      >
        <Switch enabled={enterToGenerate} setEnabled={setEnterToGenerate} />
      </SettingsRow>
    </div>
  );
};

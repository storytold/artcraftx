import { useEnterToGenerateStore } from "@storyteller/ui-promptbox";
import { Switch } from "@storyteller/ui-switch";
import { SettingsRow } from "./SettingsRow";

interface MiscSettingsPaneProps {}

export const MiscSettingsPane = (args: MiscSettingsPaneProps) => {
  const enterToGenerate = useEnterToGenerateStore((s) => s.enabled);
  const setEnterToGenerate = useEnterToGenerateStore((s) => s.setEnabled);

  return (
    <div className="text-base-fg">
      <SettingsRow
        title="Enter to generate"
        description="When on, Enter submits the prompt and Shift+Enter adds a new line. When off, both add a new line — use the button to submit."
      >
        <Switch enabled={enterToGenerate} setEnabled={setEnterToGenerate} />
      </SettingsRow>
    </div>
  );
};

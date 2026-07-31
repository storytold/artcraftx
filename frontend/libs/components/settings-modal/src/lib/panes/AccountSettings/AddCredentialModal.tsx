import { useEffect, useState } from "react";
import { Modal } from "@storyteller/ui-modal";
import { Button } from "@storyteller/ui-button";
import { Input } from "@storyteller/ui-input";
import { Select } from "@storyteller/ui-select";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faGlobe, faKey } from "@fortawesome/pro-solid-svg-icons";
import {
  API_KEY_SERVICES,
  addApiCredential,
  getServiceLogoPath,
} from "./credential-helpers";

interface AddCredentialModalProps {
  isOpen: boolean;
  onClose: () => void;
  onAdded: () => void;
  onChooseWebsiteLogin: () => void;
}

/**
 * The "Add credential" modal: pick an API key type from a dropdown and paste
 * the key (with an optional name), or switch to the website-login chooser.
 */
export const AddCredentialModal = ({
  isOpen,
  onClose,
  onAdded,
  onChooseWebsiteLogin,
}: AddCredentialModalProps) => {
  const [service, setService] = useState<string>(API_KEY_SERVICES[0].value);
  const [apiKey, setApiKey] = useState("");
  const [name, setName] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [errorMessage, setErrorMessage] = useState("");

  // Reset form each time the modal opens.
  useEffect(() => {
    if (isOpen) {
      setService(API_KEY_SERVICES[0].value);
      setApiKey("");
      setName("");
      setIsSaving(false);
      setErrorMessage("");
    }
  }, [isOpen]);

  const handleAdd = async () => {
    if (isSaving || !apiKey.trim()) return;
    setIsSaving(true);
    setErrorMessage("");
    try {
      await addApiCredential({
        service,
        apiKey: apiKey.trim(),
        name: name.trim() || undefined,
      });
      onAdded();
      onClose();
    } catch (e) {
      console.error("Failed to add credential", e);
      setErrorMessage(String(e));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title="Add credential"
      titleIcon={faKey}
      width="w-[440px]"
      showClose={true}
    >
      <div className="flex flex-col gap-4 p-1 text-base-fg">
        <div className="flex flex-col gap-1.5">
          <label className="text-sm text-base-fg/70">API key type</label>
          <div className="flex items-center gap-2.5">
            <img
              src={getServiceLogoPath(service)}
              alt=""
              className="h-6 w-6 shrink-0 object-contain icon-auto-contrast"
            />
            <Select
              className="grow"
              value={service}
              onChange={(value) => setService(String(value))}
              options={API_KEY_SERVICES.map((meta) => ({
                label: meta.label,
                value: meta.value,
              }))}
            />
          </div>
        </div>

        <div className="flex flex-col gap-1.5">
          <label className="text-sm text-base-fg/70">API key</label>
          <Input
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey((e.target as HTMLInputElement).value)}
            placeholder="Paste your API key"
          />
        </div>

        <div className="flex flex-col gap-1.5">
          <label className="text-sm text-base-fg/70">Name (optional)</label>
          <Input
            value={name}
            onChange={(e) => setName((e.target as HTMLInputElement).value)}
            placeholder="e.g. work account"
          />
        </div>

        {errorMessage && (
          <div className="text-sm text-red-400">{errorMessage}</div>
        )}

        <Button
          variant="primary"
          className="h-9"
          onClick={handleAdd}
          disabled={isSaving || !apiKey.trim()}
        >
          {isSaving ? "Adding..." : "Add"}
        </Button>

        <div className="flex items-center gap-3 text-xs text-base-fg/40">
          <div className="h-px grow bg-current opacity-30" />
          or
          <div className="h-px grow bg-current opacity-30" />
        </div>

        <Button
          variant="secondary"
          className="h-9"
          icon={faGlobe}
          onClick={onChooseWebsiteLogin}
        >
          Log into a website instead
        </Button>
      </div>
    </Modal>
  );
};

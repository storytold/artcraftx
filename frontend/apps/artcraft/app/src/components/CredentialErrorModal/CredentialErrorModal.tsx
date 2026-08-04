import { useState } from "react";
import { Modal } from "@storyteller/ui-modal";
import { Button } from "@storyteller/ui-button";
import { faUserSlash } from "@fortawesome/pro-solid-svg-icons";
import { useShowCredentialErrorModalEvent } from "@storyteller/tauri-events";

/**
 * Dismissable modal shown when the backend reports a credential problem with
 * a generation request (no account selected, unknown credential id, or a
 * credential that can't serve the request). Mounted once in MainApp; opens
 * itself on `show_credential_error_modal_event`.
 */
export const CredentialErrorModal = () => {
  const [message, setMessage] = useState<string | null>(null);

  useShowCredentialErrorModalEvent(async (event) => {
    setMessage(event.message);
  });

  return (
    <Modal
      isOpen={message !== null}
      onClose={() => setMessage(null)}
      title="Account problem"
      titleIcon={faUserSlash}
      width={440}
      showClose={true}
    >
      <div className="flex flex-col gap-4 p-1 text-base-fg">
        <p className="text-sm text-base-fg/80">{message}</p>
        <Button
          variant="primary"
          className="h-9"
          onClick={() => setMessage(null)}
        >
          Dismiss
        </Button>
      </div>
    </Modal>
  );
};

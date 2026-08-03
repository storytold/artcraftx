import { useCallback, useEffect, useState } from "react";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faUser } from "@fortawesome/pro-solid-svg-icons";
import { PopoverMenu, type PopoverItem } from "@storyteller/ui-popover";
import { Tooltip } from "@storyteller/ui-tooltip";
import { useRefreshAccountStateEvent } from "@storyteller/tauri-events";
import {
  CredentialPayload,
  getServiceLogoPath,
  getServiceMeta,
  listCredentials,
} from "@storyteller/ui-settings-modal";
import { useAccountSelectorStore } from "./accountSelectorStore";

/**
 * Toolbar account picker: lists every stored credential (API keys and
 * website/ArtCraft logins) and remembers the pick in a shared store. Rendered
 * to the left of the model selector on each generation page.
 *
 * Fetches the credential list on every mount for now — caching and cache
 * busting come later.
 */
export const AccountSelector = () => {
  const [credentials, setCredentials] = useState<CredentialPayload[]>([]);
  const selectedAccountToken = useAccountSelectorStore(
    (state) => state.selectedAccountToken
  );
  const setSelectedAccountToken = useAccountSelectorStore(
    (state) => state.setSelectedAccountToken
  );

  const refresh = useCallback(async () => {
    try {
      setCredentials(await listCredentials());
    } catch (e) {
      console.error("Failed to list accounts", e);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // Logins and credential edits emit this event; reload so the list is fresh.
  useRefreshAccountStateEvent(async () => {
    await refresh();
  });

  // Keep the selection valid: default to the first account, and re-point (or
  // clear) when the selected credential is deleted.
  useEffect(() => {
    if (credentials.length === 0) {
      if (selectedAccountToken !== null) setSelectedAccountToken(null);
      return;
    }
    const selectionExists = credentials.some(
      (credential) => credential.token === selectedAccountToken
    );
    if (!selectionExists) {
      setSelectedAccountToken(credentials[0].token);
    }
  }, [credentials, selectedAccountToken, setSelectedAccountToken]);

  const items: PopoverItem[] =
    credentials.length > 0
      ? credentials.map((credential) => ({
          label: accountLabel(credential),
          description: accountDescription(credential),
          selected: credential.token === selectedAccountToken,
          action: credential.token,
          icon: (
            <img
              src={getServiceLogoPath(credential.service)}
              alt=""
              className="h-4 w-4 object-contain icon-auto-contrast"
            />
          ),
        }))
      : [
          {
            label: "No accounts yet",
            description: "Add one in Settings → Accounts",
            selected: false,
            disabled: true,
          },
        ];

  const handleSelect = (item: PopoverItem) => {
    if (item.action) setSelectedAccountToken(item.action);
  };

  const selectedCredential = credentials.find(
    (credential) => credential.token === selectedAccountToken
  );

  return (
    <Tooltip content="Account" position="top" className="z-50" closeOnClick>
      <PopoverMenu
        items={items}
        onSelect={handleSelect}
        mode="toggle"
        richList
        panelTitle="Select Account"
        panelClassName="w-[300px]"
        buttonClassName="max-w-48"
        triggerIcon={
          <span className="flex h-4 w-4 shrink-0 items-center justify-center">
            {selectedCredential ? (
              <img
                src={getServiceLogoPath(selectedCredential.service)}
                alt=""
                className="h-4 w-4 object-contain icon-auto-contrast"
              />
            ) : (
              <FontAwesomeIcon icon={faUser} className="text-xs" />
            )}
          </span>
        }
      />
    </Tooltip>
  );
};

/** Row title: the most personal identity we have for the account. */
const accountLabel = (credential: CredentialPayload): string =>
  credential.name ||
  credential.username ||
  credential.email ||
  getServiceMeta(credential.service).label;

/** Row subtitle: the service, plus a key preview for API-key accounts. */
const accountDescription = (credential: CredentialPayload): string => {
  const serviceLabel = getServiceMeta(credential.service).label;
  if (credential.kind === "api_key") {
    return `${serviceLabel} · ${credential.api_key_preview ?? ""}${"*".repeat(8)}`;
  }
  return serviceLabel;
};

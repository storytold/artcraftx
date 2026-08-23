import { useCallback, useEffect, useState } from "react";
import { User } from "lucide-react";
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
  const selectedAccountId = useAccountSelectorStore(
    (state) => state.selectedAccountId
  );
  const setSelectedAccountId = useAccountSelectorStore(
    (state) => state.setSelectedAccountId
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
      if (selectedAccountId !== null) setSelectedAccountId(null);
      return;
    }
    const selectionExists = credentials.some(
      (credential) => credential.id === selectedAccountId
    );
    if (!selectionExists) {
      setSelectedAccountId(credentials[0].id);
    }
  }, [credentials, selectedAccountId, setSelectedAccountId]);

  const items: PopoverItem[] =
    credentials.length > 0
      ? credentials.map((credential) => ({
          label: accountLabel(credential),
          description: accountDescription(credential),
          selected: credential.id === selectedAccountId,
          action: credential.id,
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
    if (item.action) setSelectedAccountId(item.action);
  };

  const selectedCredential = credentials.find(
    (credential) => credential.id === selectedAccountId
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
              <User size="1em" className="text-xs" />
            )}
          </span>
        }
      />
    </Tooltip>
  );
};

/**
 * Auto-assigned usernames (notably Midjourney's, e.g. "u3869671173") are just
 * "u" followed by digits. They carry no human meaning, so we prefer the email
 * for these; a username that looks human-set is still preferred.
 */
const AUTO_ASSIGNED_USERNAME_PATTERN = /^u\d+$/;

/** Row title: the most personal identity we have for the account. */
const accountLabel = (credential: CredentialPayload): string => {
  const name = credential.name?.trim();
  if (name) {
    return name;
  }

  const username = credential.username?.trim();
  const email = credential.email?.trim();
  const serviceLabel = getServiceMeta(credential.service).label;

  if (username && !AUTO_ASSIGNED_USERNAME_PATTERN.test(username)) {
    return username;
  }

  // Auto-assigned (or missing) username: prefer the email, then fall back.
  return email || username || serviceLabel;
};

/** Row subtitle: the service, plus a key preview for API-key accounts. */
const accountDescription = (credential: CredentialPayload): string => {
  const serviceLabel = getServiceMeta(credential.service).label;
  if (credential.kind === "api_key") {
    return `${serviceLabel} · ${credential.api_key_preview ?? ""}${"*".repeat(8)}`;
  }
  return serviceLabel;
};

import { useCallback, useEffect, useState } from "react";
import { Button } from "@storyteller/ui-button";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faKey, faPen, faPlus, faTrash } from "@fortawesome/pro-solid-svg-icons";
import {
  CredentialPayload,
  getServiceLogoPath,
  getServiceMeta,
  listCredentials,
} from "./credential-helpers";
import { AddCredentialModal } from "./AddCredentialModal";
import { WebsiteLoginModal } from "./WebsiteLoginModal";
import { EditCredentialModal } from "./EditCredentialModal";
import { DeleteCredentialModal } from "./DeleteCredentialModal";

/**
 * The accounts pane body: lists credential files from the credentials
 * directory (starts empty for new installs) and offers add / edit / delete.
 */
export const CredentialsManager = () => {
  const [credentials, setCredentials] = useState<CredentialPayload[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isAddOpen, setIsAddOpen] = useState(false);
  const [isWebsiteLoginOpen, setIsWebsiteLoginOpen] = useState(false);
  const [editing, setEditing] = useState<CredentialPayload | null>(null);
  const [deleting, setDeleting] = useState<CredentialPayload | null>(null);

  const refresh = useCallback(async () => {
    try {
      const result = await listCredentials();
      setCredentials(result);
    } catch (e) {
      console.error("Failed to list credentials", e);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return (
    <div className="flex flex-col gap-3 text-base-fg">
      <div className="flex items-center justify-between">
        <span className="text-sm text-base-fg/60">
          API keys and website logins
        </span>
        <Button
          variant="primary"
          className="h-8"
          icon={faPlus}
          onClick={() => setIsAddOpen(true)}
        >
          Add
        </Button>
      </div>

      {isLoading ? null : credentials.length === 0 ? (
        <div className="flex flex-col items-center gap-2 rounded-lg border border-dashed border-ui-panel-border px-6 py-10 text-center">
          <FontAwesomeIcon icon={faKey} className="text-xl text-base-fg/30" />
          <p className="text-sm text-base-fg/60">No accounts yet.</p>
          <p className="text-xs text-base-fg/40">
            Add an API key or log into a website to get started.
          </p>
        </div>
      ) : (
        <div className="flex flex-col gap-1.5">
          {credentials.map((credential) => (
            <CredentialRow
              key={credential.id}
              credential={credential}
              onEdit={() => setEditing(credential)}
              onDelete={() => setDeleting(credential)}
            />
          ))}
        </div>
      )}

      <AddCredentialModal
        isOpen={isAddOpen}
        onClose={() => setIsAddOpen(false)}
        onAdded={refresh}
        onChooseWebsiteLogin={() => {
          setIsAddOpen(false);
          setIsWebsiteLoginOpen(true);
        }}
      />
      <WebsiteLoginModal
        isOpen={isWebsiteLoginOpen}
        onClose={() => setIsWebsiteLoginOpen(false)}
      />
      <EditCredentialModal
        credential={editing}
        onClose={() => setEditing(null)}
        onSaved={refresh}
      />
      <DeleteCredentialModal
        credential={deleting}
        onClose={() => setDeleting(null)}
        onDeleted={refresh}
      />
    </div>
  );
};

const CredentialRow = ({
  credential,
  onEdit,
  onDelete,
}: {
  credential: CredentialPayload;
  onEdit: () => void;
  onDelete: () => void;
}) => {
  const meta = getServiceMeta(credential.service);
  const isApiKey = credential.kind === "api_key";
  // API keys: "Service (optional name)" over a partial key (prefix +
  // asterisks). Cookies: label over whatever identity info the file carries.
  const primaryLine = isApiKey
    ? credential.name
      ? `${meta.label} (${credential.name})`
      : meta.label
    : credential.name || meta.label;
  const secondaryLine = isApiKey
    ? `${credential.api_key_preview ?? ""}${"*".repeat(12)}`
    : [credential.username, credential.email].filter(Boolean).join(" · ") ||
      "Website login";

  return (
    <div
      className={`group flex items-center gap-3 rounded-lg border border-ui-panel-border bg-ui-controls/30 px-3 py-2.5 ${
        isApiKey ? "cursor-pointer hover:bg-ui-controls/60" : ""
      }`}
      onClick={isApiKey ? onEdit : undefined}
    >
      <img
        src={getServiceLogoPath(credential.service)}
        alt={`${meta.label} logo`}
        className="h-6 w-6 shrink-0 object-contain icon-auto-contrast"
      />
      <div className="flex min-w-0 grow flex-col">
        <span className="truncate text-sm font-medium">{primaryLine}</span>
        <span
          className={`truncate text-xs text-base-fg/50 ${isApiKey ? "font-mono" : ""}`}
        >
          {secondaryLine}
        </span>
      </div>
      {isApiKey && (
        <Button
          variant="secondary"
          className="h-8 w-8 opacity-0 transition-opacity group-hover:opacity-100"
          icon={faPen}
          onClick={(e: React.MouseEvent) => {
            e.stopPropagation();
            onEdit();
          }}
        />
      )}
      <Button
        variant="secondary"
        className="h-8 w-8 opacity-0 transition-opacity group-hover:opacity-100"
        icon={faTrash}
        onClick={(e: React.MouseEvent) => {
          e.stopPropagation();
          onDelete();
        }}
      />
    </div>
  );
};

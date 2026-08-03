import { Modal } from "@storyteller/ui-modal";
import { faGlobe } from "@fortawesome/pro-solid-svg-icons";
import {
  ServiceMeta,
  WEBSITE_LOGIN_SERVICES,
  getServiceLogoPath,
  openWebLogin,
} from "./credential-helpers";

interface WebsiteLoginModalProps {
  isOpen: boolean;
  onClose: () => void;
  /** Called for services that use our own username/password form (ArtCraft). */
  onChoosePasswordLogin: (service: ServiceMeta) => void;
}

/**
 * Website login chooser: one logo button per site. Most sites open a backend
 * login window; ArtCraft services switch to a username/password form instead.
 * The modal closes so the user can complete login there.
 */
export const WebsiteLoginModal = ({
  isOpen,
  onClose,
  onChoosePasswordLogin,
}: WebsiteLoginModalProps) => {
  const handleLogin = async (meta: ServiceMeta) => {
    if (meta.passwordLogin) {
      onClose();
      onChoosePasswordLogin(meta);
      return;
    }
    if (!meta.loginWebsite) return;
    try {
      await openWebLogin(meta.loginWebsite);
      onClose();
    } catch (error) {
      console.error(`Failed to open login window for ${meta.loginWebsite}:`, error);
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title="Log into a website"
      titleIcon={faGlobe}
      width="w-[440px]"
      showClose={true}
    >
      <div className="flex flex-col gap-3 p-1 text-base-fg">
        <p className="text-sm text-base-fg/60">
          Pick a website to log into. Your session cookies will be stored as a
          credential.
        </p>
        <div className="grid grid-cols-2 gap-2.5">
          {WEBSITE_LOGIN_SERVICES.map((meta) => (
            <button
              key={meta.value}
              className="flex h-16 flex-col items-center justify-center gap-1.5 rounded-lg border border-ui-panel-border bg-ui-controls/40 transition-colors hover:bg-ui-controls"
              onClick={() => handleLogin(meta)}
            >
              <img
                src={getServiceLogoPath(meta.value)}
                alt={`${meta.label} logo`}
                className="h-6 max-w-[120px] object-contain icon-auto-contrast"
              />
              <span className="text-xs text-base-fg/70">{meta.label}</span>
            </button>
          ))}
        </div>
      </div>
    </Modal>
  );
};

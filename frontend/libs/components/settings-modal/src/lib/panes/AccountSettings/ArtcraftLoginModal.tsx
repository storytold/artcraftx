import { useEffect, useState } from "react";
import { Modal } from "@storyteller/ui-modal";
import { Button } from "@storyteller/ui-button";
import { Input } from "@storyteller/ui-input";
import { faUser } from "@fortawesome/pro-solid-svg-icons";
import {
  ArtcraftLoginError,
  ServiceMeta,
  artcraftLogin,
  getServiceLogoPath,
} from "./credential-helpers";

interface ArtcraftLoginModalProps {
  /** Which ArtCraft service to log into; null when the modal is closed. */
  service: ServiceMeta | null;
  onClose: () => void;
  onLoggedIn: () => void;
}

/**
 * Username/password login form for ArtCraft accounts (production or local
 * dev). Unlike the website logins, this hits the login API directly and
 * stores the resulting session as a new credential.
 */
export const ArtcraftLoginModal = ({
  service,
  onClose,
  onLoggedIn,
}: ArtcraftLoginModalProps) => {
  const [usernameOrEmail, setUsernameOrEmail] = useState("");
  const [password, setPassword] = useState("");
  const [isLoggingIn, setIsLoggingIn] = useState(false);
  const [errorMessage, setErrorMessage] = useState("");

  const isOpen = service !== null;

  // Reset the form each time the modal opens.
  useEffect(() => {
    if (isOpen) {
      setUsernameOrEmail("");
      setPassword("");
      setIsLoggingIn(false);
      setErrorMessage("");
    }
  }, [isOpen]);

  const canSubmit = !isLoggingIn && !!usernameOrEmail.trim() && !!password;

  const handleLogin = async () => {
    if (!service || !canSubmit) return;
    setIsLoggingIn(true);
    setErrorMessage("");
    try {
      await artcraftLogin({
        service: service.value,
        usernameOrEmail: usernameOrEmail.trim(),
        password,
      });
      onLoggedIn();
      onClose();
    } catch (e) {
      console.error(`ArtCraft login failed for ${service.value}:`, e);
      setErrorMessage(loginErrorMessage(e));
    } finally {
      setIsLoggingIn(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") handleLogin();
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={`Log into ${service?.label ?? "ArtCraft"}`}
      titleIcon={faUser}
      width="w-[440px]"
      showClose={true}
    >
      <div className="flex flex-col gap-4 p-1 text-base-fg">
        <div className="flex items-center gap-2.5">
          <img
            src={getServiceLogoPath(service?.value ?? "artcraft")}
            alt=""
            className="h-6 w-6 shrink-0 object-contain icon-auto-contrast"
          />
          <span className="text-sm text-base-fg/60">
            Sign in with your {service?.label ?? "ArtCraft"} account.
          </span>
        </div>

        <div className="flex flex-col gap-1.5">
          <label className="text-sm text-base-fg/70">Username or email</label>
          <Input
            value={usernameOrEmail}
            onChange={(e) =>
              setUsernameOrEmail((e.target as HTMLInputElement).value)
            }
            onKeyDown={handleKeyDown}
            placeholder="you@example.com"
            autoFocus
          />
        </div>

        <div className="flex flex-col gap-1.5">
          <label className="text-sm text-base-fg/70">Password</label>
          <Input
            type="password"
            value={password}
            onChange={(e) => setPassword((e.target as HTMLInputElement).value)}
            onKeyDown={handleKeyDown}
            placeholder="Password"
          />
        </div>

        {errorMessage && (
          <div className="text-sm text-red-400">{errorMessage}</div>
        )}

        <Button
          variant="primary"
          className="h-9"
          onClick={handleLogin}
          disabled={!canSubmit}
        >
          {isLoggingIn ? "Logging in..." : "Log in"}
        </Button>

        {/* Placeholders; flows not built yet. */}
        <div className="flex items-center justify-between text-xs text-base-fg/40">
          <button
            type="button"
            disabled
            title="Coming soon"
            className="cursor-not-allowed"
          >
            Create account
          </button>
          <button
            type="button"
            disabled
            title="Coming soon"
            className="cursor-not-allowed"
          >
            Forgot password?
          </button>
        </div>
      </div>
    </Modal>
  );
};

/** Map a rejected `artcraft_login_command` payload to a user-facing message. */
const loginErrorMessage = (e: unknown): string => {
  const error = e as ArtcraftLoginError;
  if (!error || typeof error !== "object" || !("error_type" in error)) {
    return String(e);
  }
  switch (error.error_type) {
    case "invalid_credentials":
      return "Invalid username or password.";
    case "account_needs_password":
      return "This account has no password yet. Please reset your password first.";
    case "server_error":
      return "Server error. Please try again.";
    case "connection_error":
      return "Could not reach the server. Check your connection and try again.";
    default:
      return error.message || String(e);
  }
};

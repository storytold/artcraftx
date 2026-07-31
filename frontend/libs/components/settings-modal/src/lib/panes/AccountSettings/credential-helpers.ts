import { invoke } from "@tauri-apps/api/core";
import { getServicesBasePath } from "@storyteller/model-list";

// Mirrors the Rust `CredentialPayload` (crates/artcraftx/src/commands/credentials).
export interface CredentialPayload {
  /** File name within the credentials directory, e.g. `fal_api_2.toml`. */
  id: string;
  service: string;
  kind: "cookies" | "api_key";
  name: string | null;
  api_key: string | null;
  api_key_preview: string | null;
  username: string | null;
  email: string | null;
  updated_at: string | null;
  failed_at: string | null;
  succeeded_at: string | null;
}

export interface ServiceMeta {
  value: string;
  label: string;
  logo: string;
  /**
   * For website-login services: the `LoginWebsite` value passed to
   * `open_web_login_command` (mirrors the Rust enum in
   * crates/artcraftx/src/credentials/login_website.rs).
   */
  loginWebsite?: string;
}

/** API-key services offered in the "Add" dropdown. */
export const API_KEY_SERVICES: ServiceMeta[] = [
  { value: "artcraft_api", label: "ArtCraft", logo: "artcraft.svg" },
  { value: "fal_api", label: "FAL", logo: "fal.svg" },
  { value: "openai_api", label: "OpenAI", logo: "openai.svg" },
  { value: "replicate_api", label: "Replicate", logo: "replicate.svg" },
  { value: "xai_api", label: "xAI", logo: "grok.svg" },
];

/** Website-login services offered as logo buttons. */
export const WEBSITE_LOGIN_SERVICES: ServiceMeta[] = [
  { value: "artcraft_cookies", label: "ArtCraft", logo: "artcraft.svg", loginWebsite: "artcraft" },
  { value: "runway_cookies", label: "Runway", logo: "runway.svg", loginWebsite: "runway" },
  { value: "higgsfield_cookies", label: "Higgsfield", logo: "higgsfield.svg", loginWebsite: "higgsfield" },
  { value: "openart_cookies", label: "OpenArt", logo: "openart.svg", loginWebsite: "openart" },
  { value: "magnific_cookies", label: "Magnific", logo: "magnific.svg", loginWebsite: "magnific" },
  { value: "xai_cookies", label: "xAI", logo: "grok.svg", loginWebsite: "xai" },
];

// Every service the backend knows about, so hand-written credential files
// for services we don't offer in the "Add" flow still render sensibly.
const ALL_SERVICES: ServiceMeta[] = [
  ...API_KEY_SERVICES,
  ...WEBSITE_LOGIN_SERVICES,
  { value: "grok_cookies", label: "Grok", logo: "grok.svg" },
  { value: "magnific_cookies", label: "Magnific", logo: "magnific.svg" },
  { value: "midjourney_cookies", label: "Midjourney", logo: "midjourney.svg" },
  { value: "sora_cookies", label: "Sora", logo: "openai.svg" },
  { value: "worldlabs_cookies", label: "World Labs", logo: "worldlabs.svg" },
];

export const getServiceMeta = (service: string): ServiceMeta => {
  const found = ALL_SERVICES.find((meta) => meta.value === service);
  if (found) return found;
  return { value: service, label: service, logo: "generic.svg" };
};

export const getServiceLogoPath = (service: string): string =>
  `${getServicesBasePath()}/${getServiceMeta(service).logo}`;

// ── Tauri command wrappers ──────────────────────────────────────────────────

export const listCredentials = async (): Promise<CredentialPayload[]> => {
  const response = await invoke<{ credentials: CredentialPayload[] }>(
    "list_credentials_command"
  );
  return response.credentials;
};

export const addApiCredential = async (args: {
  service: string;
  apiKey: string;
  name?: string;
}): Promise<CredentialPayload> => {
  const response = await invoke<{ credential: CredentialPayload }>(
    "add_api_credential_command",
    { service: args.service, apiKey: args.apiKey, name: args.name ?? null }
  );
  return response.credential;
};

export const editApiCredential = async (args: {
  fileName: string;
  apiKey?: string;
  name?: string;
}): Promise<CredentialPayload> => {
  const response = await invoke<{ credential: CredentialPayload }>(
    "edit_api_credential_command",
    {
      fileName: args.fileName,
      apiKey: args.apiKey ?? null,
      name: args.name ?? null,
    }
  );
  return response.credential;
};

export const deleteCredential = async (fileName: string): Promise<void> => {
  await invoke("delete_credentials_command", { fileName });
};

/**
 * Open a web-login window for a site. The `website` value must be a
 * `LoginWebsite` variant (see `WEBSITE_LOGIN_SERVICES[].loginWebsite`). The
 * backend opens a fresh webview and, once the user signs in, saves the
 * captured cookies as a credential and emits `refresh_account_state_event`.
 */
export const openWebLogin = async (website: string): Promise<void> => {
  await invoke("open_web_login_command", { website });
};

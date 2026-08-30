// The rules that keep a page's (model, provider, account) choice valid.
//
// Pure functions over the loaded models and accounts, so the store can apply
// them and tests can pin them. Every "which one do we swap to" decision goes
// through one of the `choose*` functions below — that's where most-recently-
// used preferences will plug in later (last provider used per model, last
// model used per provider, last account per model/provider).
import { GenerationProvider } from "@storyteller/api-enums";
import { Model } from "@storyteller/model-list";

// The identity of a stored credential the user can generate with.
export interface AccountSummary {
  id: string;
  // Backend `GenerationSource` value, e.g. "grok_cookies", "fal_api".
  service: string;
}

// The provider an account's service generates through. `undefined` for
// services the app can't generate with yet (they impose no constraint).
export const providerForService = (service: string): GenerationProvider | undefined => {
  switch (service) {
    case "artcraft":
    case "artcraft_local":
    case "artcraft_cookies":
    case "artcraft_api":
      return GenerationProvider.Artcraft;
    case "fal":
    case "fal_api":
      return GenerationProvider.Fal;
    case "grok":
    case "grok_cookies":
    case "xai_api":
      return GenerationProvider.Grok;
    case "midjourney":
    case "midjourney_cookies":
      return GenerationProvider.Midjourney;
    case "sora":
    case "sora_cookies":
    case "openai_api":
      return GenerationProvider.Sora;
    case "world_labs":
    case "worldlabs_cookies":
      return GenerationProvider.WorldLabs;
    default:
      return undefined;
  }
};

export const modelIsOfferedBy = (model: Model, provider: GenerationProvider): boolean =>
  model.getProviders().includes(provider);

// The provider to use for a model: the current one if the model is offered
// there, else the model's default. (Future: the last provider used for it.)
export const chooseProviderForModel = (
  model: Model,
  current: GenerationProvider | undefined,
): GenerationProvider | undefined => {
  if (current !== undefined && modelIsOfferedBy(model, current)) return current;
  return model.getProviders()[0];
};

// The model to switch to when a provider doesn't offer the current one: the
// first model on the page the provider does offer. (Future: the last model
// used with that provider.)
export const chooseModelForProvider = (
  provider: GenerationProvider,
  pageModels: Model[],
): Model | undefined => pageModels.find((m) => modelIsOfferedBy(m, provider));

// The account to use for a provider: the current one if it generates through
// that provider, else the first stored account that does. (Future: the last
// account used for that model/provider.)
export const chooseAccountForProvider = (
  provider: GenerationProvider,
  accounts: AccountSummary[],
  currentAccountId: string | null,
): AccountSummary | undefined => {
  const current = accounts.find((a) => a.id === currentAccountId);
  if (current && providerForService(current.service) === provider) return current;
  return accounts.find((a) => providerForService(a.service) === provider);
};

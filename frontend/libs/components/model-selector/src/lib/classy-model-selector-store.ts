// The selection state shared by every generation page: which model and
// provider each page uses, and which account (credential) the app generates
// with. Every write goes through the rules in `selection-rules.ts`, so the
// combination stays valid: a model is only ever paired with a provider that
// offers it, and with an account that generates through that provider.
import { create } from "zustand";
import { ModelPage } from "./model-pages";
import { ImageModel, Model, VideoModel } from "@storyteller/model-list";
import { GenerationProvider } from "@storyteller/api-enums";
import {
  AccountSummary,
  chooseAccountForProvider,
  chooseModelForProvider,
  chooseProviderForModel,
  modelIsOfferedBy,
  providerForService,
} from "./selection-rules";

interface ClassyModelSelectorState {
  selectedModels: { [page in ModelPage]?: Model };
  selectedProviders: { [page in ModelPage]?: { [modelId: string]: GenerationProvider } };
  // The models each page's picker offers; registered by the selectors so the
  // rules can swap to a compatible model on that page.
  pageModels: { [page in ModelPage]?: Model[] };
  // Stored credentials, and the one the toolbar account picker selected
  // (shared across pages). `null` until accounts load or when none exist.
  accounts: AccountSummary[];
  selectedAccountId: string | null;

  registerPageModels: (page: ModelPage, models: Model[]) => void;
  // Select a model. Its provider becomes the account's provider when the model
  // is offered there, else the model's default — and the account follows.
  setSelectedModel: (page: ModelPage, model: Model) => void;
  // Remember a provider for a model on a page. When that model is the page's
  // selection, the account follows the provider.
  setSelectedProvider: (page: ModelPage, modelId: string, provider: GenerationProvider) => void;
  // Select a provider for the page. If it doesn't offer the current model, the
  // model switches to one it does offer (nothing changes if there is none).
  selectProvider: (page: ModelPage, provider: GenerationProvider) => void;
  setAccounts: (accounts: AccountSummary[]) => void;
  // Select an account. Every page's model switches to one its provider
  // offers (keeping the model when it already does).
  setSelectedAccountId: (id: string | null) => void;
}

type Draft = Pick<ClassyModelSelectorState, "selectedModels" | "selectedProviders" | "selectedAccountId">;

export const useClassyModelSelectorStore = create<ClassyModelSelectorState>(
  (set, get) => ({
    selectedModels: {},
    selectedProviders: {},
    pageModels: {},
    accounts: [],
    selectedAccountId: null,

    registerPageModels: (page, models) =>
      set((state) => ({ pageModels: { ...state.pageModels, [page]: models } })),

    setSelectedModel: (page, model) =>
      set((state) => {
        const accountProvider = accountProviderOf(state);
        const provider = chooseProviderForModel(
          model,
          accountProvider ?? state.selectedProviders[page]?.[model.id],
        );
        const draft = withModel(state, page, model, provider);
        return provider ? withAccountFor(state, draft, provider) : draft;
      }),

    setSelectedProvider: (page, modelId, provider) =>
      set((state) => {
        const draft = withProvider(state, page, modelId, provider);
        const isPageSelection = state.selectedModels[page]?.id === modelId;
        return isPageSelection ? withAccountFor(state, draft, provider) : draft;
      }),

    selectProvider: (page, provider) =>
      set((state) => {
        const current = state.selectedModels[page];
        const model =
          current && modelIsOfferedBy(current, provider)
            ? current
            : chooseModelForProvider(provider, state.pageModels[page] ?? []);
        if (!model) return {}; // The provider offers nothing on this page.
        return withAccountFor(state, withModel(state, page, model, provider), provider);
      }),

    setAccounts: (accounts) => {
      set({ accounts });
      const { selectedAccountId } = get();
      const stillExists = accounts.some((a) => a.id === selectedAccountId);
      if (accounts.length === 0) {
        if (selectedAccountId !== null) get().setSelectedAccountId(null);
      } else if (!stillExists) {
        get().setSelectedAccountId(accounts[0].id);
      }
    },

    setSelectedAccountId: (id) =>
      set((state) => {
        const account = state.accounts.find((a) => a.id === id);
        const provider = account ? providerForService(account.service) : undefined;
        let draft: Draft = { ...pick(state), selectedAccountId: id };
        if (provider === undefined) return draft; // No constraint from this account.
        for (const page of Object.keys(state.selectedModels) as ModelPage[]) {
          const current = state.selectedModels[page];
          if (!current) continue;
          const model = modelIsOfferedBy(current, provider)
            ? current
            : chooseModelForProvider(provider, state.pageModels[page] ?? []);
          if (!model) continue; // Nothing on this page works with the provider; leave it.
          draft = withModel(draft, page, model, provider);
        }
        return draft;
      }),
  })
);

// ── Rule application helpers (pure) ──

const pick = (state: Draft): Draft => ({
  selectedModels: state.selectedModels,
  selectedProviders: state.selectedProviders,
  selectedAccountId: state.selectedAccountId,
});

const accountProviderOf = (state: ClassyModelSelectorState): GenerationProvider | undefined => {
  const account = state.accounts.find((a) => a.id === state.selectedAccountId);
  return account ? providerForService(account.service) : undefined;
};

const withProvider = (state: Draft, page: ModelPage, modelId: string, provider: GenerationProvider): Draft => ({
  ...pick(state),
  selectedProviders: {
    ...state.selectedProviders,
    [page]: { ...(state.selectedProviders[page] ?? {}), [modelId]: provider },
  },
});

const withModel = (state: Draft, page: ModelPage, model: Model, provider: GenerationProvider | undefined): Draft => {
  const draft: Draft = { ...pick(state), selectedModels: { ...state.selectedModels, [page]: model } };
  return provider ? withProvider(draft, page, model.id, provider) : draft;
};

// Point the account at one that generates through `provider`, if any exists.
const withAccountFor = (state: ClassyModelSelectorState, draft: Draft, provider: GenerationProvider): Draft => {
  const account = chooseAccountForProvider(provider, state.accounts, draft.selectedAccountId);
  return account ? { ...draft, selectedAccountId: account.id } : draft;
};

// ── Non-reactive getters ──

export const getSelectedImageModel = (
  page: ModelPage
): ImageModel | undefined => {
  const { selectedModels } = useClassyModelSelectorStore.getState();
  const maybeModel = selectedModels[page];
  if (!maybeModel) {
    return undefined;
  }
  // NB: We can't use "instanceof" checks with Vite minification and class name mangling.
  // We have to do type tagging a different way.
  if (maybeModel.kind === "image_model") {
    return maybeModel as ImageModel;
  }
  return undefined;
};

export const getSelectedVideoModel = (
  page: ModelPage
): VideoModel | undefined => {
  const { selectedModels } = useClassyModelSelectorStore.getState();
  const maybeModel = selectedModels[page];
  if (!maybeModel) {
    return undefined;
  }
  if (maybeModel.kind !== "video_model") {
    return undefined;
  }
  return maybeModel as VideoModel;
};

export const getSelectedProviderForModel = (
  page: ModelPage,
  modelId: string
): GenerationProvider | undefined => {
  const { selectedProviders } = useClassyModelSelectorStore.getState();
  const byPage = selectedProviders[page];
  if (!byPage) return undefined;
  return byPage[modelId];
};

// ── Reactive hooks for UI subscriptions ──

export const useSelectedModel = (page: ModelPage): Model | undefined =>
  useClassyModelSelectorStore((s) => s.selectedModels[page]);

export const useSelectedImageModel = (
  page: ModelPage
): ImageModel | undefined => {
  const maybeModel = useSelectedModel(page);
  if (!maybeModel) return undefined;
  return maybeModel.kind === "image_model"
    ? (maybeModel as ImageModel)
    : undefined;
};

export const useSelectedVideoModel = (
  page: ModelPage
): VideoModel | undefined => {
  const maybeModel = useSelectedModel(page);
  if (!maybeModel) return undefined;
  return maybeModel.kind === "video_model"
    ? (maybeModel as VideoModel)
    : undefined;
};

// TODO: This shouldn't be on a per-page basis.
export const useSelectedProviderForModel = (
  page: ModelPage,
  modelId: string | undefined
): GenerationProvider | undefined =>
  useClassyModelSelectorStore((s) =>
    modelId ? s.selectedProviders[page]?.[modelId] : undefined
  );

// The account the app generates with (shared across pages).
export const useSelectedAccountId = (): string | null =>
  useClassyModelSelectorStore((s) => s.selectedAccountId);

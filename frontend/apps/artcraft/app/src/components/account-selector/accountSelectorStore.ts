import { create } from "zustand";

interface AccountSelectorState {
  /**
   * Credential id (the credential's file name, e.g. `artcraft_2.toml`) of
   * the account selected in the toolbar account pickers. Shared across all
   * generation pages. `null` until accounts load (the selector defaults to
   * the first account) or when no accounts exist.
   */
  selectedAccountId: string | null;
  setSelectedAccountId: (id: string | null) => void;
}

export const useAccountSelectorStore = create<AccountSelectorState>((set) => ({
  selectedAccountId: null,
  setSelectedAccountId: (id) => set({ selectedAccountId: id }),
}));

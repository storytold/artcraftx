import { create } from "zustand";

interface AccountSelectorState {
  /**
   * Stable credential id (`credential_{entropy}`) of the account selected
   * in the toolbar account pickers. Shared across all generation pages.
   * `null` until accounts load (the selector defaults to the first account)
   * or when no accounts exist.
   */
  selectedAccountId: string | null;
  setSelectedAccountId: (id: string | null) => void;
}

export const useAccountSelectorStore = create<AccountSelectorState>((set) => ({
  selectedAccountId: null,
  setSelectedAccountId: (id) => set({ selectedAccountId: id }),
}));

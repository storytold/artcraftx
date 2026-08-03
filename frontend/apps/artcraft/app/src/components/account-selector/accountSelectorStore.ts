import { create } from "zustand";

interface AccountSelectorState {
  /**
   * Stable credential token (`credential_{entropy}`) of the account selected
   * in the toolbar account pickers. Shared across all generation pages.
   * `null` until accounts load (the selector defaults to the first account)
   * or when no accounts exist.
   */
  selectedAccountToken: string | null;
  setSelectedAccountToken: (token: string | null) => void;
}

export const useAccountSelectorStore = create<AccountSelectorState>((set) => ({
  selectedAccountToken: null,
  setSelectedAccountToken: (token) => set({ selectedAccountToken: token }),
}));

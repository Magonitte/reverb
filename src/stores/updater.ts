import { create } from "zustand";

/** Estado do atualizador do app. Vazio até a F06 (`updater://available`, `updater://progress`). */
interface UpdaterState {
  available: boolean;
  version: string | null;
}

export const useUpdaterStore = create<UpdaterState>(() => ({ available: false, version: null }));

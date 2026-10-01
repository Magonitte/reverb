import { create } from "zustand";
import type { AppInfo } from "@/bindings/AppInfo";
import { api } from "@/lib/ipc/api";

interface AppInfoState {
  info: AppInfo | null;
  load: () => Promise<void>;
}

export const useAppInfoStore = create<AppInfoState>((set) => ({
  info: null,
  load: async () => set({ info: await api.appInfo() }),
}));

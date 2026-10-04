import { create } from "zustand";
import type { CoreError } from "@/bindings/CoreError";
import type { SettingsPatch } from "@/bindings/SettingsPatch";
import type { SettingsView } from "@/bindings/SettingsView";
import { api } from "@/lib/ipc/api";
import { onEvent } from "@/lib/ipc/events";
import { DEFAULT_NOTIFICATIONS, useNotificationPreferences } from "./notificationPreferences";

interface SettingsState {
  settings: SettingsView | null;
  loading: boolean;
  error: CoreError | null;
  load: () => Promise<void>;
  /** Atualização otimista: aplica já; em erro desfaz só as chaves alteradas e relança. */
  update: (patch: SettingsPatch) => Promise<void>;
  reset: () => Promise<void>;
}

type Bag = Record<string, unknown>;

/** Aplica no `SettingsView` só as chaves que ele conhece (segredos nunca chegam à UI). */
function mergePatch(view: SettingsView, patch: SettingsPatch): SettingsView {
  const next: Bag = { ...view };
  for (const [key, value] of Object.entries(patch)) {
    if (value !== undefined && value !== null && key in view && key !== "secretsStatus") {
      next[key] = value;
    }
  }
  return next as unknown as SettingsView;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: null,
  loading: false,
  error: null,

  load: async () => {
    set({ loading: true, error: null });
    try {
      set({ settings: await api.settingsGet(), loading: false });
    } catch (e) {
      set({ loading: false, error: e as CoreError });
    }
  },

  update: async (patch) => {
    const before = get().settings;
    if (before) set({ settings: mergePatch(before, patch) });
    try {
      set({ settings: await api.settingsUpdate(patch), error: null });
    } catch (e) {
      const current = get().settings;
      if (before && current) {
        const restored: Bag = { ...current };
        for (const key of Object.keys(patch)) {
          if (key in before) restored[key] = (before as unknown as Bag)[key];
        }
        set({ settings: restored as unknown as SettingsView });
      }
      set({ error: e as CoreError });
      throw e;
    }
  },

  reset: async () => {
    set({ settings: await api.settingsReset() });
    useNotificationPreferences.getState().update(DEFAULT_NOTIFICATIONS);
  },
}));

/** Hidrata a store e passa a aplicar `settings://changed`. Devolve a função de cancelamento. */
export async function initSettingsStore(): Promise<() => void> {
  const unlisten = await onEvent<SettingsView>("settings://changed", (view) => {
    useSettingsStore.setState({ settings: view });
  });
  await useSettingsStore.getState().load();
  return unlisten;
}

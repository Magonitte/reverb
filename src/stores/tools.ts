import { create } from "zustand";
import type { Tool } from "@/bindings/Tool";
import type { ToolChanged } from "@/bindings/ToolChanged";
import type { ToolProgress } from "@/bindings/ToolProgress";
import type { ToolStatus } from "@/bindings/ToolStatus";
import { api } from "@/lib/ipc/api";
import { onEvent } from "@/lib/ipc/events";
import i18n from "@/lib/i18n";
import { errorText } from "@/lib/errors";

interface ToolsState {
  statuses: ToolStatus[];
  progress: Partial<Record<Tool, ToolProgress>>;
  updating: Partial<Record<Tool, boolean>>;
  errors: Partial<Record<Tool, string>>;
  load: () => Promise<void>;
  updateTool: (tool: Tool) => Promise<void>;
}

export const useToolsStore = create<ToolsState>((set, get) => ({
  statuses: [],
  progress: {},
  updating: {},
  errors: {},
  load: async () => set({ statuses: await api.toolsStatus() }),
  updateTool: async (tool) => {
    if (get().updating[tool] || get().progress[tool]) return;
    set((s) => ({
      updating: { ...s.updating, [tool]: true },
      errors: { ...s.errors, [tool]: undefined },
    }));
    try {
      await api.toolsUpdate(tool);
      await get().load();
    } catch (e) {
      set((s) => ({ errors: { ...s.errors, [tool]: errorText(i18n.t, e) } }));
      throw e;
    } finally {
      set((s) => ({
        updating: { ...s.updating, [tool]: false },
        progress: { ...s.progress, [tool]: undefined },
      }));
    }
  },
}));

export function toolsUpdateCount(statuses: ToolStatus[]): number {
  return statuses.filter((s) => s.updateAvailable).length;
}

export async function initToolsStore(): Promise<() => void> {
  const offs = await Promise.all([
    onEvent<{ tool: Tool; error: unknown }>("tools://failed", ({ tool, error }) => {
      useToolsStore.setState((s) => ({
        progress: { ...s.progress, [tool]: undefined },
        errors: { ...s.errors, [tool]: errorText(i18n.t, error) },
      }));
    }),
    onEvent<ToolProgress>("tools://progress", (p) =>
      useToolsStore.setState((s) => ({
        progress: { ...s.progress, [p.tool]: p },
      })),
    ),
    onEvent<ToolChanged>("tools://changed", ({ tool }) => {
      useToolsStore.setState((s) => ({ progress: { ...s.progress, [tool]: undefined } }));
      void useToolsStore
        .getState()
        .load()
        .catch((e) => {
          useToolsStore.setState((s) => ({
            errors: { ...s.errors, [tool]: errorText(i18n.t, e) },
          }));
        });
    }),
  ]);
  await useToolsStore.getState().load();
  return () => offs.forEach((off) => off());
}

import { create } from "zustand";
import type { Tool } from "@/bindings/Tool";
import type { ToolChanged } from "@/bindings/ToolChanged";
import type { ToolProgress } from "@/bindings/ToolProgress";
import type { ToolStatus } from "@/bindings/ToolStatus";
import { api } from "@/lib/ipc/api";
import { onEvent } from "@/lib/ipc/events";

interface ToolsState {
  statuses: ToolStatus[];
  progress: Partial<Record<Tool, ToolProgress>>;
  load: () => Promise<void>;
}

export const useToolsStore = create<ToolsState>((set) => ({
  statuses: [],
  progress: {},
  load: async () => set({ statuses: await api.toolsStatus() }),
}));

export function toolsUpdateCount(statuses: ToolStatus[]): number {
  return statuses.filter((s) => s.updateAvailable).length;
}

export async function initToolsStore(): Promise<() => void> {
  const offs = await Promise.all([
    onEvent<ToolProgress>("tools://progress", (p) =>
      useToolsStore.setState((s) => ({
        progress:
          p.percent >= 100
            ? { ...s.progress, [p.tool]: undefined }
            : { ...s.progress, [p.tool]: p },
      })),
    ),
    onEvent<ToolChanged>("tools://changed", () => void useToolsStore.getState().load()),
  ]);
  await useToolsStore.getState().load();
  return () => offs.forEach((off) => off());
}

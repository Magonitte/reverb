import { create } from "zustand";
import type { Sync } from "@/bindings/Sync";
import { api } from "@/lib/ipc/api";
import { onEvent } from "@/lib/ipc/events";
import i18n from "@/lib/i18n";
import { useUiStore } from "./ui";

let ticket = 0;
interface SyncState {
  syncs: Sync[];
  loading: boolean;
  error: unknown;
  load: () => Promise<void>;
}
export const useSyncsStore = create<SyncState>((set) => ({
  syncs: [],
  loading: false,
  error: null,
  load: async () => {
    const mine = ++ticket;
    set({ loading: true, error: null });
    try {
      const syncs = await api.syncsList();
      if (mine === ticket) set({ syncs, loading: false });
    } catch (error) {
      if (mine === ticket) set({ error, loading: false });
    }
  },
}));
export function resetSyncsStore(): void {
  ++ticket;
  useSyncsStore.setState({ syncs: [], loading: false, error: null });
}
export async function initSyncsStore(): Promise<() => void> {
  const off = await onEvent<{ id: string }>("sync://updated", ({ id }) => {
    const previous = useSyncsStore.getState().syncs.find((sync) => sync.id === id);
    void useSyncsStore
      .getState()
      .load()
      .then(() => {
        const sync = useSyncsStore.getState().syncs.find((sync) => sync.id === id);
        if (previous?.lastResult?.running && sync?.lastResult && !sync.lastResult.running) {
          useUiStore
            .getState()
            .pushToast({
              message: i18n.t("sync.completed", {
                title: sync.title,
                count: sync.lastResult.added,
                failed: sync.lastResult.failed,
              }),
              tone: sync.lastResult.failed || sync.lastResult.error ? "warning" : "success",
            });
        }
      });
  });
  await useSyncsStore.getState().load();
  return off;
}

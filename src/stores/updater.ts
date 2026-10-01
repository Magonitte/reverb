import { create } from "zustand";
import i18n from "@/lib/i18n";
import { api } from "@/lib/ipc/api";
import { onEvent } from "@/lib/ipc/events";
import type { AppUpdateInfo, UpdateProgress } from "@/lib/updater";
import { useToolsStore } from "./tools";
import { useUiStore } from "./ui";

export type UpdaterPhase =
  | "idle"
  | "checking"
  | "uptodate"
  | "available"
  | "downloading"
  | "ready"
  | "restarting"
  | "error";

interface UpdaterState {
  phase: UpdaterPhase;
  available: boolean;
  version: string | null;
  currentVersion: string | null;
  notes: string | null;
  date: string | null;
  downloaded: number;
  total: number | null;
  error: string | null;
  check: () => Promise<void>;
  install: () => Promise<void>;
}

export const initialUpdaterState = {
  phase: "idle" as const,
  available: false,
  version: null,
  currentVersion: null,
  notes: null,
  date: null,
  downloaded: 0,
  total: null,
  error: null,
};

function errMessage(error: unknown): string {
  if (typeof error === "string" && error) return error;
  if (error && typeof error === "object" && "message" in error) {
    const message = (error as { message: unknown }).message;
    if (typeof message === "string" && message) return message;
  }
  return i18n.t("updater.error");
}

function showAvailable(info: AppUpdateInfo) {
  useUpdaterStore.setState({
    phase: "available",
    available: true,
    version: info.version,
    currentVersion: info.currentVersion,
    notes: info.notes,
    date: info.date,
    error: null,
  });
  useUiStore.getState().pushToast({
    message: i18n.t("updater.toastAvailable", { version: info.version }),
    tone: "info",
  });
}

export const useUpdaterStore = create<UpdaterState>((set) => ({
  ...initialUpdaterState,
  check: async () => {
    set({ phase: "checking", error: null });
    try {
      const [info] = await Promise.all([
        api.updaterCheck(),
        api.toolsCheckUpdates(true).then(() => useToolsStore.getState().load()),
      ]);
      if (info) showAvailable(info);
      else {
        set({
          phase: "uptodate",
          available: false,
          version: null,
          notes: null,
          date: null,
          error: null,
        });
      }
    } catch (error) {
      set({ phase: "error", error: errMessage(error), available: false });
    }
  },
  install: async () => {
    set({ phase: "downloading", downloaded: 0, total: null, error: null });
    try {
      await api.updaterInstall();
      set({ phase: "ready" });
      await api.appRestart();
      set({ phase: "restarting" });
    } catch (error) {
      set({ phase: "error", error: errMessage(error) });
    }
  },
}));

export async function initUpdaterStore(): Promise<() => void> {
  const offs = await Promise.all([
    onEvent<AppUpdateInfo>("updater://available", (info) => showAvailable(info)),
    onEvent<UpdateProgress>("updater://progress", (progress) =>
      useUpdaterStore.setState((state) => ({
        phase: state.phase === "restarting" || state.phase === "ready" ? state.phase : "downloading",
        downloaded: progress.downloaded,
        total: progress.total,
        available: true,
      })),
    ),
  ]);
  return () => offs.forEach((off) => off());
}

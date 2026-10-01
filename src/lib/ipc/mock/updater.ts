import type { AppUpdateInfo } from "@/lib/updater";
import { mockBus } from "./bus";

export type MockUpdaterMode = "none" | "available" | "error";

export const MOCK_UPDATE: AppUpdateInfo = {
  version: "0.1.1",
  currentVersion: "0.1.0",
  notes: "Correção do atualizador.",
  date: "2026-10-01T00:00:00Z",
};

let mode: MockUpdaterMode = "none";
let checks = 0;

export function setMockUpdaterMode(next: MockUpdaterMode) {
  mode = next;
}

export function resetMockUpdater() {
  mode = "none";
  checks = 0;
}

export function mockUpdaterCheckCount() {
  return checks;
}

export async function mockUpdaterCheck(): Promise<AppUpdateInfo | null> {
  checks += 1;
  if (mode === "error") throw { kind: "network", message: "Sem conexão com o GitHub" };
  if (mode === "available") return MOCK_UPDATE;
  return null;
}

export async function mockUpdaterInstall(): Promise<void> {
  if (mode === "error") throw { kind: "network", message: "Falha ao baixar a atualização" };
  mockBus.emit("updater://progress", { downloaded: 40, total: 100 });
  await new Promise((resolve) => setTimeout(resolve, 400));
  mockBus.emit("updater://progress", { downloaded: 100, total: 100 });
}

export function mockAppRestart(): void {
  return undefined;
}

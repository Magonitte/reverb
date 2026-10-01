import { invoke } from "@tauri-apps/api/core";
import type { AppInfo } from "@/bindings/AppInfo";
import type { SettingsPatch } from "@/bindings/SettingsPatch";
import type { InstallOutcome } from "@/bindings/InstallOutcome";
import type { SettingsView } from "@/bindings/SettingsView";
import type { Tool } from "@/bindings/Tool";
import type { ToolStatus } from "@/bindings/ToolStatus";
import type { UpdateInfo } from "@/bindings/UpdateInfo";
import { isTauri } from "./isTauri";

export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (import.meta.env.VITE_REVERB_MOCK === "1" || !isTauri()) {
    const { mockCall } = await import("./mock");
    return mockCall<T>(cmd, args);
  }
  return invoke<T>(cmd, args);
}

export const api = {
  appInfo: () => call<AppInfo>("app_info"),
  settingsGet: () => call<SettingsView>("settings_get"),
  settingsUpdate: (patch: SettingsPatch) => call<SettingsView>("settings_update", { patch }),
  settingsReset: () => call<SettingsView>("settings_reset"),
  toolsStatus: () => call<ToolStatus[]>("tools_status"),
  toolsInstallMissing: () => call<Tool[]>("tools_install_missing"),
  toolsCheckUpdates: (force = false) => call<UpdateInfo[]>("tools_check_updates", { force }),
  toolsUpdate: (tool: Tool) => call<InstallOutcome>("tools_update", { tool }),
  toolsRollback: (tool: Tool) => call<string>("tools_rollback", { tool }),
};

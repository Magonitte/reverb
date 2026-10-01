import { invoke } from "@tauri-apps/api/core";
import type { AppInfo } from "@/bindings/AppInfo";
import type { DuplicateHit } from "@/bindings/DuplicateHit";
import type { EnqueueRequest } from "@/bindings/EnqueueRequest";
import type { Job } from "@/bindings/Job";
import type { MoveTarget } from "@/bindings/MoveTarget";
import type { QueueState } from "@/bindings/QueueState";
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
  enqueue: (request: Partial<EnqueueRequest> & { url: string }) =>
    call<Job>("enqueue", { request }),
  checkDuplicates: (sourceIds: string[], profileId?: string) =>
    call<DuplicateHit[]>("check_duplicates", { sourceIds, profileId }),
  jobsList: () => call<Job[]>("jobs_list"),
  jobCancel: (id: string) => call<void>("job_cancel", { id }),
  jobRetry: (id: string) => call<Job>("job_retry", { id }),
  jobRemove: (id: string) => call<void>("job_remove", { id }),
  jobMove: (id: string, target: MoveTarget) => call<void>("job_move", { id, target }),
  jobsClearFinished: () => call<number>("jobs_clear_finished"),
  queuePause: () => call<void>("queue_pause"),
  queueResume: () => call<void>("queue_resume"),
  queueState: () => call<QueueState>("queue_state"),
  jobsCancelAll: () => call<void>("jobs_cancel_all"),
};

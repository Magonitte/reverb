import { invoke } from "@tauri-apps/api/core";
import type { Analysis } from "@/bindings/Analysis";
import type { AppInfo } from "@/bindings/AppInfo";
import type { Candidate } from "@/bindings/Candidate";
import type { DuplicateHit } from "@/bindings/DuplicateHit";
import type { EnqueueRequest } from "@/bindings/EnqueueRequest";
import type { Job } from "@/bindings/Job";
import type { MetadataResult } from "@/bindings/MetadataResult";
import type { MoveTarget } from "@/bindings/MoveTarget";
import type { OfficialMatch } from "@/bindings/OfficialMatch";
import type { PreviewRequest } from "@/bindings/PreviewRequest";
import type { QueueState } from "@/bindings/QueueState";
import type { SearchResult } from "@/bindings/SearchResult";
import type { SettingsPatch } from "@/bindings/SettingsPatch";
import type { InstallOutcome } from "@/bindings/InstallOutcome";
import type { SettingsView } from "@/bindings/SettingsView";
import type { Tool } from "@/bindings/Tool";
import type { ToolStatus } from "@/bindings/ToolStatus";
import type { UpdateInfo } from "@/bindings/UpdateInfo";
import type { UrlKind } from "@/bindings/UrlKind";
import type { VideoInfo } from "@/bindings/VideoInfo";
import type { AppUpdateInfo } from "@/lib/updater";
import { isTauri } from "./isTauri";

export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (import.meta.env.VITE_REVERB_MOCK === "1" || !isTauri()) {
    const { mockCall } = await import("./mock");
    return mockCall<T>(cmd, args);
  }
  return invoke<T>(cmd, args);
}

export const COMMANDS = [
  "app_info",
  "settings_get",
  "settings_update",
  "settings_reset",
  "tools_status",
  "tools_install_missing",
  "tools_check_updates",
  "tools_update",
  "tools_rollback",
  "url_classify",
  "analyze",
  "search",
  "pick_folder",
  "open_output_dir",
  "clipboard_read_text",
  "library_reveal",
  "library_open_file",
  "library_cover",
  "template_preview",
  "find_official_version",
  "metadata_preview",
  "metadata_search",
  "enqueue",
  "check_duplicates",
  "jobs_list",
  "job_cancel",
  "job_retry",
  "job_remove",
  "job_move",
  "jobs_clear_finished",
  "queue_pause",
  "queue_resume",
  "queue_state",
  "jobs_cancel_all",
  "updater_check",
  "updater_install",
  "app_restart",
] as const;

export type SearchSource = "ytmusic" | "youtube";

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
  urlClassify: (input: string) => call<UrlKind>("url_classify", { input }),
  analyze: (url: string) => call<Analysis>("analyze", { url }),
  search: (source: SearchSource, query: string) =>
    call<SearchResult[]>("search", { source, query }),
  pickFolder: () => call<string | null>("pick_folder"),
  openOutputDir: () => call<void>("open_output_dir"),
  clipboardReadText: () => call<string>("clipboard_read_text"),
  libraryReveal: (path: string) => call<void>("library_reveal", { path }),
  libraryOpenFile: (path: string) => call<void>("library_open_file", { path }),
  libraryCover: (id: number) => call<string | null>("library_cover", { id }),
  templatePreview: (template: string) => call<string>("template_preview", { template }),
  findOfficialVersion: (video: VideoInfo, isrc?: string) =>
    call<OfficialMatch | null>("find_official_version", { video, isrc: isrc ?? null }),
  metadataPreview: (request: PreviewRequest) =>
    call<MetadataResult>("metadata_preview", { request }),
  metadataSearch: (query: string) => call<Candidate[]>("metadata_search", { query }),
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
  updaterCheck: () => call<AppUpdateInfo | null>("updater_check"),
  updaterInstall: () => call<void>("updater_install"),
  appRestart: () => call<void>("app_restart"),
};

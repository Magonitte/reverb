import type { NativeNotice } from "@/stores/nativeNotices";
import type { ImportAnalysis } from "@/bindings/ImportAnalysis";
import type { ImportSelection } from "@/bindings/ImportSelection";
import type { ArtistHit } from "@/bindings/ArtistHit";
import type { ArtistOptions } from "@/bindings/ArtistOptions";
import type { FollowedArtist } from "@/bindings/FollowedArtist";
import type { ArtistRelease } from "@/bindings/ArtistRelease";
import type { UpgradeCandidate } from "@/bindings/UpgradeCandidate";
import type { CookiesTestResult } from "@/bindings/CookiesTestResult";
import { invoke } from "@tauri-apps/api/core";
import type { Analysis } from "@/bindings/Analysis";
import type { DiagnosticReport } from "@/bindings/DiagnosticReport";
import type { DataPaths } from "@/bindings/DataPaths";
import type { JsRuntime } from "@/bindings/JsRuntime";
import type { AppInfo } from "@/bindings/AppInfo";
import type { Candidate } from "@/bindings/Candidate";
import type { DuplicateHit } from "@/bindings/DuplicateHit";
import type { EnqueueRequest } from "@/bindings/EnqueueRequest";
import type { Job } from "@/bindings/Job";
import type { ImportReport } from "@/bindings/ImportReport";
import type { TrackTags } from "@/bindings/TrackTags";
import type { TagCover } from "@/bindings/TagCover";
import type { LibraryItem } from "@/bindings/LibraryItem";
import type { LibraryPage } from "@/bindings/LibraryPage";
import type { LibraryQuery } from "@/bindings/LibraryQuery";
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
import type { Sync } from "@/bindings/Sync";
import type { SyncCreate } from "@/bindings/SyncCreate";
import type { SyncUpdate } from "@/bindings/SyncUpdate";
import type { SyncItem } from "@/bindings/SyncItem";
import type { SyncResult } from "@/bindings/SyncResult";
import { isTauri } from "./isTauri";
import type { LosslessReport } from "@/bindings/LosslessReport";

export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (import.meta.env.VITE_REVERB_MOCK === "1" || !isTauri()) {
    const { mockCall } = await import("./mock");
    return mockCall<T>(cmd, args);
  }
  return invoke<T>(cmd, args);
}

export const COMMANDS = [
  "notifications_pending",
  "notifications_resize",
  "notifications_hide",
  "import_analyze",
  "import_enqueue",
  "artists_search",
  "artist_follow",
  "artist_update",
  "artist_unfollow",
  "artists_followed",
  "artist_releases",
  "artists_check_now",
  "missing_list",
  "missing_download",
  "verify_lossless",
  "open_source_url",
  "audio_duration",
  "upgrade_scan",
  "upgrade_enqueue",
  "provider_test",
  "waveform",
  "trim_audio",
  "cookies_test",
  "diagnostics_run",
  "diagnostics_last",
  "logs_export",
  "data_export",
  "data_import",
  "open_data_dir",
  "data_paths",
  "pick_backup_path",
  "runtime_choices",
  "bookmarklet_code",
  "bookmarklet_copy",
  "deeplink_test",
  "syncs_list",
  "sync_create",
  "sync_update",
  "sync_delete",
  "sync_run",
  "sync_items",
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
  "library_list",
  "library_import",
  "library_rescan",
  "tags_read",
  "tags_write",
  "pick_audio_file",
  "pick_image_file",
  "artwork_read",
  "artwork_fetch",
  "library_get",
  "library_artists",
  "library_albums",
  "library_delete",
  "library_clear",
  "review_dismiss",
  "review_list",
  "review_apply",
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

export type SearchSource = "ytmusic" | "youtube" | "archive" | "jamendo";

export const api = {
  notificationsPending: () => call<NativeNotice[]>("notifications_pending"),
  notificationsResize: (height: number) => call<void>("notifications_resize", { height }),
  notificationsHide: () => call<void>("notifications_hide"),
  importAnalyze: (url: string) => call<ImportAnalysis>("import_analyze", { url }),
  importEnqueue: (selection: ImportSelection) => call<Job[]>("import_enqueue", { selection }),
  artistsSearch: (name: string) => call<ArtistHit[]>("artists_search", { name }),
  artistFollow: (providerArtistId: string, options: ArtistOptions) =>
    call<FollowedArtist>("artist_follow", { providerArtistId, options }),
  artistUpdate: (id: string, options: ArtistOptions) =>
    call<void>("artist_update", { id, options }),
  artistUnfollow: (id: string, deleteFiles: boolean) =>
    call<void>("artist_unfollow", { id, deleteFiles }),
  artistsFollowed: () => call<FollowedArtist[]>("artists_followed"),
  artistReleases: (id: string) => call<ArtistRelease[]>("artist_releases", { id }),
  artistsCheckNow: () => call<Job[]>("artists_check_now"),
  missingList: (artistId?: string) => call<ArtistRelease[]>("missing_list", { artistId }),
  missingDownload: (releaseIds: string[]) => call<Job[]>("missing_download", { releaseIds }),
  verifyLossless: (id: number) => call<LosslessReport>("verify_lossless", { id }),
  openSourceUrl: (url: string) => call<void>("open_source_url", { url }),
  audioDuration: (path: string) => call<number>("audio_duration", { path }),
  upgradeScan: (ids?: number[]) => call<UpgradeCandidate[]>("upgrade_scan", { ids }),
  upgradeEnqueue: (ids: number[]) => call<Job[]>("upgrade_enqueue", { ids }),
  providerTest: (provider: string) => call<void>("provider_test", { provider }),
  waveform: (path: string) => call<string>("waveform", { path }),
  trimAudio: (path: string, startS: number, endS: number) =>
    call<void>("trim_audio", { path, startS, endS }),
  cookiesTest: () => call<CookiesTestResult>("cookies_test"),
  diagnosticsRun: () => call<DiagnosticReport>("diagnostics_run"),
  diagnosticsLast: () => call<DiagnosticReport | null>("diagnostics_last"),
  logsExport: () => call<string | null>("logs_export"),
  dataExport: (path: string) => call<void>("data_export", { path }),
  dataImport: (path: string) => call<string>("data_import", { path }),
  openDataDir: () => call<void>("open_data_dir"),
  dataPaths: () => call<DataPaths>("data_paths"),
  pickBackupPath: (restore = false) => call<string | null>("pick_backup_path", { restore }),
  runtimeChoices: () => call<JsRuntime[]>("runtime_choices"),
  bookmarkletCode: () => call<string>("bookmarklet_code"),
  bookmarkletCopy: () => call<void>("bookmarklet_copy"),
  deeplinkTest: (link: string) => call<void>("deeplink_test", { link }),
  syncsList: () => call<Sync[]>("syncs_list"),
  syncCreate: (request: SyncCreate) => call<Sync>("sync_create", { request }),
  syncUpdate: (id: string, request: SyncUpdate) => call<Sync>("sync_update", { id, request }),
  syncDelete: (id: string, deleteFiles = false) => call<void>("sync_delete", { id, deleteFiles }),
  syncRun: (id: string) => call<SyncResult>("sync_run", { id }),
  syncItems: (id: string) => call<SyncItem[]>("sync_items", { id }),
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
  libraryCover: (id: number, size = 256) => call<string | null>("library_cover", { id, size }),
  templatePreview: (template: string) => call<string>("template_preview", { template }),
  libraryList: (query: Partial<LibraryQuery> = {}) => call<LibraryPage>("library_list", { query }),
  libraryImport: (paths: string[]) => call<ImportReport>("library_import", { paths }),
  libraryRescan: () => call<ImportReport>("library_rescan"),
  tagsRead: (path: string) => call<TrackTags>("tags_read", { path }),
  tagsWrite: (path: string, tags: TrackTags, reorganize = false) =>
    call<string>("tags_write", { path, tags, reorganize }),
  pickAudioFile: () => call<string | null>("pick_audio_file"),
  pickImageFile: () => call<string | null>("pick_image_file"),
  artworkRead: (path: string) => call<TagCover>("artwork_read", { path }),
  artworkFetch: (url: string) => call<TagCover>("artwork_fetch", { url }),
  libraryGet: (id: number) => call<LibraryItem | null>("library_get", { id }),
  libraryArtists: () => call<string[]>("library_artists"),
  libraryAlbums: (artist?: string) => call<string[]>("library_albums", { artist: artist ?? null }),
  libraryDelete: (ids: number[], deleteFiles = false) =>
    call<number>("library_delete", { ids, deleteFiles }),
  libraryClear: () => call<number>("library_clear"),
  reviewDismiss: (id: number) => call<void>("review_dismiss", { id }),
  reviewList: (offset = 0) => call<LibraryPage>("review_list", { offset }),
  reviewApply: (id: number, candidate: Candidate | null, manual: TrackTags | null = null) =>
    call<LibraryItem>("review_apply", { id, candidate, manual }),
  findOfficialVersion: (video: VideoInfo, isrc?: string) =>
    call<OfficialMatch | null>("find_official_version", { video, isrc: isrc ?? null }),
  metadataPreview: (request: PreviewRequest) =>
    call<MetadataResult>("metadata_preview", { request }),
  metadataSearch: (query: string) => call<Candidate[]>("metadata_search", { query }),
  enqueue: (request: Partial<EnqueueRequest> & { url: string }) =>
    call<Job>("enqueue", { request }),
  checkDuplicates: (
    sourceIds: string[],
    profileId?: string,
    identity?: { acoustidId?: string; mbRecordingId?: string },
  ) => call<DuplicateHit[]>("check_duplicates", { sourceIds, profileId, ...identity }),
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

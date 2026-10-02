import type { AppInfo } from "@/bindings/AppInfo";
import { BOOKMARKLET, mockBookmarkletCopy, mockDeepLinkTest } from "./integration";
import type { SyncCreate } from "@/bindings/SyncCreate";
import type { SyncUpdate } from "@/bindings/SyncUpdate";
import {
  mockSyncsList,
  mockSyncCreate,
  mockSyncUpdate,
  mockSyncDelete,
  mockSyncRun,
  mockSyncItems,
} from "./syncs";
export { resetMockSyncs } from "./syncs";
import type { LibraryQuery } from "@/bindings/LibraryQuery";
import type { TrackTags } from "@/bindings/TrackTags";
import type { Candidate } from "@/bindings/Candidate";
import { mockReviewApply } from "./library";
import {
  mockLibraryImport,
  mockLibraryRescan,
  mockTagsRead,
  mockTagsWrite,
  mockPickAudioFile,
  mockPickImageFile,
  mockArtworkRead,
  mockArtworkFetch,
} from "./library";
import {
  mockLibraryList,
  mockLibraryGet,
  mockLibraryArtists,
  mockLibraryAlbums,
  mockLibraryDelete,
  mockLibraryClear,
  mockReviewDismiss,
} from "./library";
import type { EnqueueRequest } from "@/bindings/EnqueueRequest";
import type { MoveTarget } from "@/bindings/MoveTarget";
import type { PreviewRequest } from "@/bindings/PreviewRequest";
import type { SettingsPatch } from "@/bindings/SettingsPatch";
import type { Tool } from "@/bindings/Tool";
import type { VideoInfo } from "@/bindings/VideoInfo";
import {
  mockAnalyze,
  mockClipboardReadText,
  mockLibraryReveal,
  mockOpenOutputDir,
  mockPickFolder,
  mockSearch,
  mockUrlClassify,
} from "./media";
import { mockFindOfficialVersion, mockMetadataPreview, mockMetadataSearch } from "./metadata";
import { mockTemplatePreview, mockLibraryCover, mockLibraryOpenFile } from "./postprocess";
import {
  mockCheckDuplicates,
  mockEnqueue,
  mockJobCancel,
  mockJobMove,
  mockJobRemove,
  mockJobRetry,
  mockJobsCancelAll,
  mockJobsClearFinished,
  mockJobsList,
  mockQueuePause,
  mockQueueResume,
  mockQueueState,
} from "./queue";
import { mockSettingsGet, mockSettingsReset, mockSettingsUpdate } from "./settings";
import {
  mockToolsCheckUpdates,
  mockToolsInstallMissing,
  mockToolsRollback,
  mockToolsStatus,
  mockToolsUpdate,
} from "./tools";
import { mockAppRestart, mockUpdaterCheck, mockUpdaterInstall } from "./updater";

export { mockBus } from "./bus";
export { mockCalls, resetMockMedia, setMockClipboard } from "./media";
export { resetMockQueue } from "./queue";
export { applyScenario, scenarioFromLocation, startMock } from "./scenarios";
export { resetMockSettings } from "./settings";
export { resetMockTools, seedMockTool, seedMockToolVersions } from "./tools";
export { resetMockUpdater, setMockUpdaterMode } from "./updater";

const handlers: Record<string, (args?: Record<string, unknown>) => unknown> = {
  syncs_list: () => mockSyncsList(),
  sync_create: (args) => mockSyncCreate(args?.request as SyncCreate),
  sync_update: (args) => mockSyncUpdate(args?.id as string, args?.request as SyncUpdate),
  sync_delete: (args) => mockSyncDelete(args?.id as string, Boolean(args?.deleteFiles)),
  sync_run: (args) => mockSyncRun(args?.id as string),
  sync_items: (args) => mockSyncItems(args?.id as string),
  app_info: (): AppInfo => ({ version: "0.1.0", platform: "windows", arch: "x86_64" }),
  bookmarklet_code: () => BOOKMARKLET,
  bookmarklet_copy: () => mockBookmarkletCopy(),
  deeplink_test: (args) => mockDeepLinkTest(args?.link as string),
  settings_get: () => mockSettingsGet(),
  settings_update: (args) => mockSettingsUpdate((args?.patch ?? {}) as SettingsPatch),
  settings_reset: () => mockSettingsReset(),
  tools_status: () => mockToolsStatus(),
  tools_install_missing: () => mockToolsInstallMissing(),
  tools_check_updates: (args) => mockToolsCheckUpdates(Boolean(args?.force)),
  tools_update: (args) => mockToolsUpdate(args?.tool as Tool),
  tools_rollback: (args) => mockToolsRollback(args?.tool as Tool),
  url_classify: (args) => mockUrlClassify(args?.input as string),
  analyze: (args) => mockAnalyze(args?.url as string),
  search: (args) => mockSearch(args?.source as string, args?.query as string),
  pick_folder: () => mockPickFolder(),
  open_output_dir: () => mockOpenOutputDir(),
  clipboard_read_text: () => mockClipboardReadText(),
  library_reveal: (args) => mockLibraryReveal(args?.path as string),
  library_open_file: (args) => mockLibraryOpenFile(args?.path as string),
  library_cover: (args) => mockLibraryCover(args?.id as number, args?.size as number | undefined),
  template_preview: (args) => mockTemplatePreview(args?.template as string),
  library_list: (args) => mockLibraryList(args?.query as Partial<LibraryQuery>),
  library_import: (args) => mockLibraryImport(args?.paths as string[]),
  library_rescan: () => mockLibraryRescan(),
  tags_read: (args) => mockTagsRead(args?.path as string),
  tags_write: (args) =>
    mockTagsWrite(args?.path as string, args?.tags as TrackTags, Boolean(args?.reorganize)),
  pick_audio_file: () => mockPickAudioFile(),
  pick_image_file: () => mockPickImageFile(),
  artwork_read: (args) => mockArtworkRead(args?.path as string),
  artwork_fetch: (args) => mockArtworkFetch(args?.url as string),
  library_get: (args) => mockLibraryGet(args?.id as number),
  library_artists: () => mockLibraryArtists(),
  library_albums: (args) => mockLibraryAlbums(args?.artist as string | null),
  library_delete: (args) => mockLibraryDelete(args?.ids as number[], Boolean(args?.deleteFiles)),
  library_clear: () => mockLibraryClear(),
  review_dismiss: (args) => mockReviewDismiss(args?.id as number),
  review_list: (args) => mockLibraryList({ needsReview: true, offset: args?.offset as number }),
  review_apply: (args) =>
    mockReviewApply(
      args?.id as number,
      args?.candidate as Candidate | null,
      args?.manual as TrackTags | null,
    ),
  find_official_version: (args) =>
    mockFindOfficialVersion(args?.video as VideoInfo, args?.isrc as string | null | undefined),
  metadata_preview: (args) => mockMetadataPreview(args?.request as PreviewRequest),
  metadata_search: (args) => mockMetadataSearch(args?.query as string),
  enqueue: (args) => mockEnqueue(args?.request as EnqueueRequest),
  check_duplicates: (args) =>
    mockCheckDuplicates(args?.sourceIds as string[], args?.profileId as string | undefined),
  jobs_list: () => mockJobsList(),
  job_cancel: (args) => mockJobCancel(args?.id as string),
  job_retry: (args) => mockJobRetry(args?.id as string),
  job_remove: (args) => mockJobRemove(args?.id as string),
  job_move: (args) => mockJobMove(args?.id as string, args?.target as MoveTarget),
  jobs_clear_finished: () => mockJobsClearFinished(),
  queue_pause: () => mockQueuePause(),
  queue_resume: () => mockQueueResume(),
  queue_state: () => mockQueueState(),
  jobs_cancel_all: () => mockJobsCancelAll(),
  updater_check: () => mockUpdaterCheck(),
  updater_install: () => mockUpdaterInstall(),
  app_restart: () => mockAppRestart(),
};

export async function mockCall<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const handler = handlers[cmd];
  if (!handler) throw { kind: "mock", message: `Comando sem mock: ${cmd}` };
  return handler(args) as T;
}

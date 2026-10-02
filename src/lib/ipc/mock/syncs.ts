import type { Sync } from "@/bindings/Sync";
import type { SyncCreate } from "@/bindings/SyncCreate";
import type { SyncUpdate } from "@/bindings/SyncUpdate";
import type { SyncItem } from "@/bindings/SyncItem";
import type { SyncResult } from "@/bindings/SyncResult";
import type { Job } from "@/bindings/Job";
import { mockBus } from "./bus";
import { mockAnalyze, mockCalls } from "./media";
import { mockEnqueue, mockJobCancel, seedMockJobs } from "./queue";
import { FX4_ALBUM } from "./fixtures";
import { mockLibraryMatch, mockLibraryDelete, mockLibraryGet } from "./library";
import { PROFILE_OPTIONS } from "@/lib/profiles";

let syncs: Sync[] = [];
let items: SyncItem[] = [];
let serial = 0;
const customTitles = new Set<string>();
const now = () => Math.floor(Date.now() / 1000);
const result = (): SyncResult => ({
  added: 0,
  removed: 0,
  failed: 0,
  duplicates: [],
  unavailable: 0,
  running: false,
  error: null,
});
function find(id: string): Sync {
  const sync = syncs.find((sync) => sync.id === id);
  if (!sync) throw { kind: "not_found", message: "Playlist not found" };
  return sync;
}
function changed(id: string): void {
  mockBus.emit("sync://updated", { id });
}
function validate(request: SyncCreate | SyncUpdate): void {
  if (!PROFILE_OPTIONS.some((profile) => profile.id === request.profileId))
    throw { kind: "invalid", message: "Unknown download profile" };
  if (request.maxItems !== null && (!Number.isInteger(request.maxItems) || request.maxItems < 1))
    throw { kind: "invalid", message: "Invalid item limit" };
  if (
    !Number.isInteger(request.intervalHours) ||
    request.intervalHours < 0 ||
    request.intervalHours > 8760
  )
    throw { kind: "invalid", message: "Invalid interval" };
}
export function resetMockSyncs(): void {
  syncs = [];
  items = [];
  serial = 0;
  customTitles.clear();
}
export function mockSyncsList(): Sync[] {
  return structuredClone(
    syncs.map((sync) => ({
      ...sync,
      itemCount: items.filter((item) => item.syncId === sync.id && item.state === "present").length,
    })),
  );
}
export function mockSyncItems(id: string): SyncItem[] {
  find(id);
  return structuredClone(
    items.filter((item) => item.syncId === id).sort((a, b) => a.position - b.position),
  );
}
export async function mockSyncCreate(request: SyncCreate): Promise<Sync> {
  validate(request);
  const analysis = await mockAnalyze(request.url);
  if (analysis.type !== "collection") throw { kind: "invalid", message: "Collection required" };
  if (syncs.some((sync) => sync.url === request.url))
    throw { kind: "duplicate", message: "Playlist already synchronized" };
  const sync: Sync = {
    ...request,
    id: `sync-${++serial}`,
    title: request.title?.trim() || analysis.info.title || request.url,
    playlistId: analysis.info.id,
    thumbnail: analysis.info.thumbnail,
    provider: "youtube",
    enabled: true,
    lastSyncAt: null,
    lastResult: null,
    createdAt: now(),
    itemCount: 0,
  };
  if (request.title?.trim()) customTitles.add(sync.id);
  syncs.push(sync);
  mockCalls.push({ cmd: "sync_create", args: { request } });
  changed(sync.id);
  return structuredClone(sync);
}
export function mockSyncUpdate(id: string, request: SyncUpdate): Sync {
  validate(request);
  const sync = find(id);
  if (sync.lastResult?.running) throw { kind: "busy", message: "Playlist is synchronizing" };
  if (!request.title.trim()) throw { kind: "invalid", message: "Title required" };
  if (request.title !== sync.title) customTitles.add(id);
  Object.assign(sync, request);
  mockCalls.push({ cmd: "sync_update", args: { id, request } });
  changed(id);
  return structuredClone(sync);
}
export function mockSyncDelete(id: string, deleteFiles: boolean): void {
  find(id);
  const owned = items.filter((item) => item.syncId === id);
  for (const item of owned)
    if (item.jobId && ["queued", "running"].includes(item.jobStatus ?? ""))
      mockJobCancel(item.jobId);
  if (deleteFiles)
    mockLibraryDelete(
      owned.flatMap((item) => (item.libraryId === null ? [] : [item.libraryId])),
      true,
    );
  syncs = syncs.filter((sync) => sync.id !== id);
  items = items.filter((item) => item.syncId !== id);
  customTitles.delete(id);
  mockCalls.push({ cmd: "sync_delete", args: { id, deleteFiles } });
  changed(id);
}
export async function mockSyncRun(id: string): Promise<SyncResult> {
  const sync = find(id);
  if (sync.lastResult?.running) throw { kind: "busy", message: "Playlist is synchronizing" };
  const analysis = await mockAnalyze(sync.url);
  if (analysis.type !== "collection") throw { kind: "invalid", message: "Collection required" };
  if (!customTitles.has(id) && analysis.info.title) sync.title = analysis.info.title;
  const next = result();
  const seen = new Set<string>();
  const tracks = analysis.info.entries.slice(0, sync.maxItems ?? undefined);
  for (const [index, track] of tracks.entries()) {
    if (!track.id || ["[Private video]", "[Deleted video]"].includes(track.title ?? "")) {
      next.unavailable++;
      if (track.id) seen.add(track.id);
      continue;
    }
    if (seen.has(track.id)) {
      next.duplicates.push(track.id);
      continue;
    }
    seen.add(track.id);
    let item = items.find((item) => item.syncId === id && item.sourceId === track.id);
    if (!item || item.state === "removed") next.added++;
    if (!item) {
      item = {
        syncId: id,
        sourceId: track.id,
        position: index + 1,
        title: track.title,
        state: "present",
        jobId: null,
        libraryId: null,
        jobStatus: null,
        filePath: null,
        missing: false,
      };
      items.push(item);
    }
    item.position = index + 1;
    item.state = "present";
    item.title = track.title;
    const existing = mockLibraryMatch("youtube", track.id, sync.profileId);
    const linked = item.libraryId === null ? null : mockLibraryGet(item.libraryId);
    if (existing || (linked?.profileId === sync.profileId && !linked.missing)) {
      const library = existing ?? linked!;
      item.libraryId = library.id;
      item.filePath = library.filePath;
      item.jobStatus = "done";
    } else if (!["queued", "running"].includes(item.jobStatus ?? "")) {
      const job = mockEnqueue({
        url: `https://www.youtube.com/watch?v=${track.id}`,
        sourceId: track.id,
        title: track.title ?? undefined,
        profileId: sync.profileId,
        allowDuplicate: true,
        metadataOverride: null,
        priority: false,
        playlistCtx: {
          playlistTitle: sync.title,
          playlistId: sync.playlistId ?? id,
          index: index + 1,
          syncId: id,
        },
        options: { outputDir: sync.outputDir ?? undefined },
      });
      item.jobId = job.id;
      item.libraryId = null;
      item.jobStatus = "queued";
    }
  }
  for (const item of items.filter(
    (item) => item.syncId === id && item.state === "present" && !seen.has(item.sourceId),
  )) {
    next.removed++;
    item.state = "removed";
    if (item.jobId && ["queued", "running"].includes(item.jobStatus ?? ""))
      mockJobCancel(item.jobId);
    if (sync.removeDeleted && item.libraryId !== null) {
      mockLibraryDelete([item.libraryId], true);
      item.libraryId = null;
      item.filePath = null;
    }
  }
  next.running = items.some(
    (item) =>
      item.syncId === id &&
      item.state === "present" &&
      ["queued", "running"].includes(item.jobStatus ?? ""),
  );
  sync.lastResult = next;
  sync.lastSyncAt = now();
  mockCalls.push({ cmd: "sync_run", args: { id } });
  changed(id);
  return structuredClone(next);
}
export function syncJobUpdated(job: Job): void {
  if (!job.syncId) return;
  const sync = syncs.find((sync) => sync.id === job.syncId);
  if (!sync) return;
  const item = items.find((item) => item.jobId === job.id);
  if (item) {
    item.jobStatus = job.status;
    item.libraryId = job.libraryId;
    item.filePath = job.outputPath;
  }
  if (sync.lastResult) {
    const current = items.filter((item) => item.syncId === sync.id && item.state === "present");
    sync.lastResult.running = current.some((item) =>
      ["queued", "running"].includes(item.jobStatus ?? ""),
    );
    sync.lastResult.failed = current.filter(
      (item) => item.jobStatus === "failed" || item.jobStatus === "cancelled",
    ).length;
  }
  changed(sync.id);
}
export async function seedMockSyncs(): Promise<void> {
  const created = await mockSyncCreate({
    url: "https://www.youtube.com/playlist?list=OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE",
    title: null,
    profileId: "original",
    outputDir: null,
    intervalHours: 24,
    maxItems: null,
    removeDeleted: false,
    writeM3u: true,
  });
  const sync = find(created.id);
  sync.createdAt = 1790942400;
  sync.lastSyncAt = 1790942400;
  for (const [index, track] of FX4_ALBUM.entries.entries()) {
    const status: Job["status"] | null = (["done", null, "failed", "cancelled"] as const)[
      index % 4
    ];
    const item: SyncItem = {
      syncId: sync.id,
      sourceId: track.id,
      position: index + 1,
      title: track.title,
      state: index === FX4_ALBUM.entries.length - 1 ? "removed" : "present",
      jobId: null,
      libraryId: null,
      jobStatus: status,
      filePath: null,
      missing: false,
    };
    items.push(item);
    if (status !== null) {
      const job = seedMockJobs([
        {
          sourceUrl: `https://www.youtube.com/watch?v=${track.id}`,
          sourceId: track.id,
          title: track.title,
          status,
          syncId: sync.id,
          outputPath:
            status === "done"
              ? `C:/Music/Rick Astley/${String(index + 1).padStart(2, "0")} - ${track.title}.opus`
              : null,
        },
      ])[0];
      item.jobId = job.id;
      syncJobUpdated(job);
    }
  }
  sync.lastResult = { ...result(), added: 10, failed: 4 };
  changed(sync.id);
}

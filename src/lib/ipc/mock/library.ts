import type { LibraryItem } from "@/bindings/LibraryItem";
import type { TrackTags } from "@/bindings/TrackTags";
import type { ImportReport } from "@/bindings/ImportReport";
import type { Candidate } from "@/bindings/Candidate";
import type { Job } from "@/bindings/Job";
import { mockSettingsGet } from "./settings";
import type { LibraryPage } from "@/bindings/LibraryPage";
import type { LibraryQuery } from "@/bindings/LibraryQuery";
import { mockBus } from "./bus";
import { mockCalls } from "./media";

let items: LibraryItem[] = [];
const tagsByPath = new Map<string, TrackTags>();
let pickedAudio: string | null = "C:/Musicas/example.opus";
let pickedImage: string | null = "C:/Pictures/cover.jpg";
export const setMockPickedAudio = (path: string | null) => {
  pickedAudio = path;
};
export const setMockPickedImage = (path: string | null) => {
  pickedImage = path;
};
export const mockPickAudioFile = () => pickedAudio;
export const mockPickImageFile = () => pickedImage;
export function resetMockLibrary(): void {
  items = [];
  tagsByPath.clear();
  pickedAudio = "C:/Musicas/example.opus";
  pickedImage = "C:/Pictures/cover.jpg";
}

export function mockTagsRead(path: string): TrackTags {
  if (!path) throw { kind: "file_missing", message: "File missing" };
  const item = items.find((i) => i.filePath === path);
  if (item?.missing) throw { kind: "file_missing", message: "File missing" };
  return structuredClone(
    tagsByPath.get(path) ?? {
      title:
        item?.title ??
        path
          .split(/[\\/]/)
          .pop()
          ?.replace(/\.[^.]+$/, "") ??
        "",
      artist: item?.artist ?? null,
      album: item?.album ?? null,
      albumArtist: item?.albumArtist ?? null,
      year: item?.year ?? null,
      genre: item?.genre ?? null,
      trackNo: item?.trackNo ?? null,
      trackTotal: item?.trackTotal ?? null,
      discNo: item?.discNo ?? null,
      lyrics: null,
      comment: null,
      isrc: item?.isrc ?? null,
      cover: null,
      replayGainTrackGain: null,
      replayGainTrackPeak: null,
      r128TrackGain: null,
    },
  );
}
export function mockTagsWrite(path: string, tags: TrackTags, reorganize: boolean): string {
  mockCalls.push({ cmd: "tags_write", args: { path, tags, reorganize } });
  const item = items.find((i) => i.filePath === path);
  let target = path;
  const settings = mockSettingsGet();
  if (item && reorganize && settings.autoOrganize) {
    const artist =
      tags.artist || (settings.language === "en" ? "Unknown artist" : "Artista desconhecido");
    const fields: Record<string, string | number | null> = {
      artist,
      albumartist: tags.albumArtist ?? artist,
      album: tags.album ?? "Singles",
      title: tags.title,
      track: tags.trackNo,
      disc: tags.discNo,
      year: tags.year,
      genre: tags.genre,
      channel: item.artist,
      source_id: item.sourceId,
      playlist: null,
      playlist_index: null,
    };
    const model = settings.fileTemplate.replace(
      /\{([^}:]+)(?::(\d+))?\}/g,
      (_, name: string, width: string) =>
        String(fields[name] ?? "")
          .replace(/[\\/]/g, "_")
          .padStart(Number(width ?? 0), "0"),
    );
    const relative = model
      .split(/[\\/]/)
      .filter(Boolean)
      .map((part) => part.replace(/[<>:"|?*]/g, "_").trim())
      .join("/");
    target = `${(settings.outputDir ?? "C:/Musicas/Reverb").replace(/[\\/]+$/, "")}/${relative}.${path.split(".").pop()}`;
    let suffix = 2;
    const base = target;
    while (items.some((i) => i.id !== item.id && i.filePath === target))
      target = base.replace(/(\.[^.]+)$/, ` (${suffix++})$1`);
  }
  tagsByPath.delete(path);
  tagsByPath.set(target, structuredClone(tags));
  if (item)
    Object.assign(item, {
      title: tags.title,
      filePath: target,
      artist: tags.artist,
      album: tags.album,
      albumArtist: tags.albumArtist,
      year: tags.year,
      genre: tags.genre,
      trackNo: tags.trackNo,
      trackTotal: tags.trackTotal,
      discNo: tags.discNo,
      isrc: tags.isrc,
      hasLyrics: !!tags.lyrics,
      hasSyncedLyrics: /^\[\d+:\d+/m.test(tags.lyrics ?? ""),
      updatedAt: Math.floor(Date.now() / 1000),
    });
  mockBus.emit("library://changed", {});
  return target;
}
export function mockPublishJob(job: Job): number {
  const id = Math.max(0, ...items.map((item) => item.id)) + 1;
  seedMockLibrary([
    ...items,
    {
      id,
      filePath: job.outputPath ?? `C:/Musicas/Reverb/${job.id}.opus`,
      title: job.title ?? job.sourceUrl,
      artist: job.artist,
      profileId: job.profileId,
      sourceUrl: job.sourceUrl,
      sourceId: job.sourceId,
      confidence: job.confidence,
      needsReview: job.metadataResult?.bucket === "review",
      reviewCandidates: job.metadataResult?.candidates ?? null,
    },
  ]);
  return id;
}
export function mockLibraryImport(paths: string[]): ImportReport {
  mockCalls.push({ cmd: "library_import", args: { paths } });
  let imported = 0,
    skipped = 0;
  const selected = paths.flatMap((path) =>
    /\.(mp3|m4a|opus|ogg|flac|wav)$/i.test(path)
      ? [path]
      : [`${path}/imported-1.opus`, `${path}/imported-2.flac`],
  );
  for (const path of selected) {
    if (items.some((i) => i.filePath === path)) {
      skipped++;
      continue;
    }
    const tags = mockTagsRead(path);
    const previous = items;
    const id = Math.max(0, ...previous.map((i) => i.id)) + 1;
    seedMockLibrary([...previous, { id, filePath: path, title: tags.title, origin: "import" }]);
    imported++;
  }
  mockBus.emit("library://import-progress", {
    processed: selected.length,
    total: selected.length,
    imported,
    skipped,
    failed: 0,
  });
  mockBus.emit("library://changed", {});
  return { imported, skipped, failures: [] };
}
export function mockLibraryRescan(): ImportReport {
  mockCalls.push({ cmd: "library_rescan" });
  mockBus.emit("library://changed", {});
  return { imported: 0, skipped: items.length, failures: [] };
}
export function mockArtworkRead(path: string) {
  if (!path) throw { kind: "artwork_decode", message: "Image missing" };
  mockCalls.push({ cmd: "artwork_read", args: { path } });
  return mockCover();
}
export function mockArtworkFetch(url: string) {
  if (!/^https?:\/\//.test(url)) throw { kind: "invalid", message: "Invalid image URL" };
  mockCalls.push({ cmd: "artwork_fetch", args: { url } });
  return mockCover();
}
function mockCover() {
  return {
    mimeType: "image/png",
    data: Array.from(
      atob(
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jD1sAAAAASUVORK5CYII=",
      ),
      (char) => char.charCodeAt(0),
    ),
  };
}
export function seedMockLibrary(rows: Array<Partial<LibraryItem>>): void {
  const now = Math.floor(Date.now() / 1000);
  items = rows.map((row, index) => ({
    id: index + 1,
    filePath: `C:/Musicas/Reverb/track-${index + 1}.opus`,
    missing: false,
    title: `Track ${index + 1}`,
    artist: null,
    album: null,
    albumArtist: null,
    year: null,
    genre: null,
    trackNo: null,
    trackTotal: null,
    discNo: null,
    durationS: null,
    codec: "opus",
    bitrateKbps: null,
    sourceAbrKbps: null,
    reviewCandidates: null,
    acoustidId: null,
    mbRecordingId: null,
    losslessVerdict: null,
    profileId: "original",
    provider: "youtube",
    sourceId: null,
    sourceUrl: null,
    isrc: null,
    contentType: "music",
    metadataSource: null,
    confidence: null,
    needsReview: false,
    hasLyrics: false,
    hasSyncedLyrics: false,
    coverSource: null,
    replaygainDb: null,
    origin: "download",
    addedAt: now,
    updatedAt: now,
    ...row,
  }));
  mockBus.emit("library://changed", {});
}
const normalize = (text: string) => text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
export function mockLibraryList(query: Partial<LibraryQuery> = {}): LibraryPage {
  const limit = query.limit ?? 100;
  if (limit < 1 || limit > 500) throw { kind: "invalid", message: "Invalid library limit" };
  if (
    query.format &&
    !["mp3", "m4a", "opus", "ogg", "flac", "wav"].includes(query.format.toLowerCase())
  )
    throw { kind: "invalid", message: "Invalid library format" };
  const offset = query.offset ?? 0;
  const terms = normalize(query.text ?? "").match(/[\p{L}\p{N}]+/gu) ?? [];
  const start = new Date();
  start.setHours(0, 0, 0, 0);
  const days = { today: 0, week: 6, month: 29 };
  if (query.dateRange && query.dateRange !== "all")
    start.setDate(start.getDate() - days[query.dateRange]);
  const filtered = items.filter((item) => {
    const words =
      normalize(`${item.title} ${item.artist ?? ""} ${item.album ?? ""}`).match(
        /[\p{L}\p{N}]+/gu,
      ) ?? [];
    if (
      query.text?.trim() &&
      (!terms.length || !terms.every((term) => words.some((w) => w.startsWith(term))))
    )
      return false;
    return (
      (query.artist == null || item.artist === query.artist) &&
      (query.album == null || item.album === query.album) &&
      (!query.format || item.filePath.toLowerCase().endsWith(`.${query.format.toLowerCase()}`)) &&
      (query.needsReview == null || item.needsReview === query.needsReview) &&
      (query.missing == null || item.missing === query.missing) &&
      (!query.dateRange || query.dateRange === "all" || item.addedAt >= start.getTime() / 1000)
    );
  });
  const cmp = (a: string | null, b: string | null) =>
    (a ?? "").toLowerCase().localeCompare((b ?? "").toLowerCase());
  filtered.sort((a, b) => {
    switch (query.sort) {
      case "title":
        return cmp(a.title, b.title) || a.id - b.id;
      case "artist":
        return cmp(a.artist, b.artist) || cmp(a.title, b.title) || a.id - b.id;
      case "album":
        return (
          cmp(a.album, b.album) ||
          (a.discNo ?? 0) - (b.discNo ?? 0) ||
          (a.trackNo ?? 0) - (b.trackNo ?? 0) ||
          cmp(a.title, b.title) ||
          a.id - b.id
        );
      default:
        return b.addedAt - a.addedAt || b.id - a.id;
    }
  });
  return structuredClone({
    items: filtered.slice(offset, offset + limit),
    total: filtered.length,
    offset,
    limit,
  });
}

export function mockLibraryMatch(provider:string,sourceId:string,profileId:string):LibraryItem|null {
  const item=items.find((item)=>item.provider===provider && item.sourceId===sourceId && item.profileId===profileId && !item.missing);
  return item?structuredClone(item):null;
}
export function mockLibraryGet(id: number): LibraryItem | null {
  return structuredClone(items.find((i) => i.id === id) ?? null);
}
export function mockLibraryArtists(): string[] {
  return [...new Set(items.map((i) => i.artist).filter((v): v is string => !!v?.trim()))].sort();
}
export function mockLibraryAlbums(artist?: string | null): string[] {
  return [
    ...new Set(
      items
        .filter((i) => artist == null || i.artist === artist)
        .map((i) => i.album)
        .filter((v): v is string => !!v?.trim()),
    ),
  ].sort();
}
export function mockLibraryDelete(ids: number[], deleteFiles: boolean): number {
  mockCalls.push({ cmd: "library_delete", args: { ids, deleteFiles } });
  const before = items.length;
  items = items.filter((i) => !ids.includes(i.id));
  mockBus.emit("library://changed", {});
  return before - items.length;
}
export function mockLibraryClear(): number {
  mockCalls.push({ cmd: "library_clear" });
  const count = items.length;
  items = [];
  mockBus.emit("library://changed", {});
  return count;
}
export function mockReviewDismiss(id: number): void {
  const item = items.find((i) => i.id === id);
  if (!item) throw { kind: "not_found", message: "Library item not found" };
  item.needsReview = false;
  item.updatedAt = Math.floor(Date.now() / 1000);
  mockBus.emit("library://changed", {});
}

export function mockReviewApply(
  id: number,
  candidate: Candidate | null,
  manual: TrackTags | null,
): LibraryItem {
  const item = items.find((i) => i.id === id);
  if (!item) throw { kind: "not_found", message: "Item missing" };
  if (!!candidate === !!manual) throw { kind: "invalid", message: "Choose one metadata source" };
  mockCalls.push({ cmd: "review_apply", args: { id, candidate, manual } });
  if (manual) mockTagsWrite(item.filePath, manual, true);
  else if (candidate) {
    const original = mockTagsRead(item.filePath);
    mockTagsWrite(
      item.filePath,
      {
        ...original,
        title: candidate.title,
        artist: candidate.artists.length ? candidate.artists.join(", ") : original.artist,
        album: candidate.album ?? original.album,
        albumArtist: candidate.albumArtist ?? original.albumArtist,
        year: candidate.year ?? original.year,
        trackNo: candidate.trackNo ?? original.trackNo,
        trackTotal: candidate.trackTotal ?? original.trackTotal,
        discNo: candidate.discNo ?? original.discNo,
        isrc: candidate.isrc ?? original.isrc,
        genre: candidate.genre ?? original.genre,
        cover: candidate.coverUrl ? mockArtworkFetch(candidate.coverUrl) : original.cover,
      },
      true,
    );
  }
  item.metadataSource = candidate?.provider ?? "user";
  item.needsReview = false;
  item.reviewCandidates = null;
  item.confidence = 1;
  mockBus.emit("library://changed", {});
  return structuredClone(item);
}

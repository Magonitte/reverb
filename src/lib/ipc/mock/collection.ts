import type { ImportAnalysis } from "@/bindings/ImportAnalysis";
import type { ImportSelection } from "@/bindings/ImportSelection";
import type { ArtistOptions } from "@/bindings/ArtistOptions";
import type { FollowedArtist } from "@/bindings/FollowedArtist";
import type { ArtistRelease } from "@/bindings/ArtistRelease";
import { mockEnqueue } from "./queue";
import { mockCalls } from "./media";
import { mockBus } from "./bus";
import { mockSettingsGet } from "./settings";
import { mockSyncCreate, mockSyncRun } from "./syncs";
const fields = {
  title: "Never Gonna Give You Up",
  artist: "Rick Astley",
  artists: ["Rick Astley"],
  album: "Whenever You Need Somebody",
  albumArtist: "Rick Astley",
  year: 1987,
  genre: null,
  trackNo: 1,
  trackTotal: 10,
  discNo: 1,
  coverUrl: null,
  mbRecordingId: null,
};
let artists: FollowedArtist[] = [];
const analyses = new Map<string, ImportAnalysis>();
export function resetMockCollection() {
  artists = [];
  analyses.clear();
}
export function mockImportAnalyze(url: string): ImportAnalysis {
  mockCalls.push({ cmd: "import_analyze", args: { url } });
  if (url.includes("spotify") && !mockSettingsGet().secretsStatus.spotify)
    throw { kind: "spotify_credentials_missing" };
  const provider = url.includes("spotify") ? "spotify" : "deezer";
  const collection = {
    provider,
    id: "5207214368",
    url,
    title: "Imported collection",
    cover: null,
    checksum: "fixture",
    tracks: [0, 1, 2].map((i) => ({
      id: String(i + 1),
      fields: { ...fields, title: i === 0 ? fields.title : `Track ${i + 1}`, trackNo: i + 1 },
      durationS: 214,
      isrc: i === 0 ? "GBARL9300135" : null,
      explicit: false,
    })),
  };
  const analysis: ImportAnalysis = {
    collection,
    items: collection.tracks.map((track, i) => ({
      track,
      matched: {
        videoId: i < 2 ? "lYBUbBu4W08" : null,
        confidence: i === 0 ? 1 : i === 1 ? 0.82 : 0,
        via: i === 0 ? "isrc" : "text",
        bucket: i === 0 ? "ok" : i === 1 ? "review" : "none",
      },
    })),
  };
  analyses.set(url, analysis);
  mockBus.emit("import://progress", { processed: 3, total: 3, url });
  return structuredClone(analysis);
}
export async function mockImportEnqueue(selection: ImportSelection) {
  mockCalls.push({ cmd: "import_enqueue", args: { selection } });
  const a = analyses.get(selection.url);
  if (!a) throw { kind: "import_expired" };
  if (selection.mode === "sync") {
    if (!selection.syncOptions) throw { kind: "invalid" };
    const sync = await mockSyncCreate(selection.syncOptions);
    await mockSyncRun(sync.id);
    return [];
  }
  return a.items
    .filter((i) => selection.trackIds.includes(i.track.id) && i.matched.videoId)
    .map((i, index) =>
      mockEnqueue({
        url: `https://music.youtube.com/watch?v=${i.matched.videoId}`,
        sourceId: i.matched.videoId ?? undefined,
        profileId: selection.profileId,
        metadataOverride: { ...i.track.fields, isrc: i.track.isrc },
        playlistCtx: {
          playlistTitle: a.collection.title,
          playlistId: a.collection.id,
          index: index + 1,
        },
        priority: false,
        allowDuplicate: true,
      }),
    );
}
export function mockArtistFollow(providerArtistId: string, options: ArtistOptions) {
  mockCalls.push({ cmd: "artist_follow", args: { providerArtistId, options } });
  const a = {
    id: `artist-${providerArtistId}`,
    providerArtistId,
    name: "Rick Astley",
    picture: null,
    options,
    lastCheckAt: null,
  };
  artists.push(a);
  mockBus.emit("artists://updated", {});
  return a;
}
export function mockArtistsFollowed() {
  return structuredClone(artists);
}
export function mockArtistUpdate(id: string, options: ArtistOptions) {
  const a = artists.find((a) => a.id === id);
  if (!a) throw { kind: "not_found" };
  a.options = options;
  mockBus.emit("artists://updated", {});
}
export function mockArtistUnfollow(id: string) {
  artists = artists.filter((a) => a.id !== id);
  mockBus.emit("artists://updated", {});
}
export function mockArtistReleases(id?: string): ArtistRelease[] {
  return artists
    .filter((a) => !id || a.id === id)
    .flatMap((a) =>
      Array.from({ length: 10 }, (_, i) => ({
        artistId: a.id,
        id: String(100 + i),
        title: i === 0 ? fields.album : `Release ${i + 1}`,
        recordType: "album",
        releaseDate: "1987-11-12",
        cover: null,
        monitored: i < 8,
        tracks: [{ id: "1", fields, durationS: 214, isrc: "GBARL9300135", explicit: false }],
        present: i < 3 ? 1 : 0,
        total: 1,
        state: i >= 8 ? "unmonitored" : i < 3 ? "complete" : "absent",
      })),
    );
}
export function mockArtistsCheck() {
  artists.forEach((a) => {
    a.lastCheckAt = Math.floor(Date.now() / 1000);
  });
  mockBus.emit("artists://updated", {});
  return [];
}
export function mockMissingDownload(releaseIds: string[]) {
  mockCalls.push({ cmd: "missing_download", args: { releaseIds } });
  return [
    mockEnqueue({
      url: "https://music.youtube.com/watch?v=lYBUbBu4W08",
      priority: false,
      allowDuplicate: true,
      metadataOverride: { ...fields, isrc: "GBARL9300135" },
    }),
  ];
}

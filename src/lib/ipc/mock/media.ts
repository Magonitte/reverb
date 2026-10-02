import type { Analysis } from "@/bindings/Analysis";
import type { SearchResult } from "@/bindings/SearchResult";
import type { UrlKind } from "@/bindings/UrlKind";
import type { VideoInfo } from "@/bindings/VideoInfo";
import { hintKind } from "@/lib/urlkind";
import { FX1_VIDEO, FX2_MUSIC, FX3_CLIP, FX4_ALBUM, FX7_SEARCH } from "./fixtures";

/** Chamadas das ações de sistema, para os testes conferirem (ex.: `library_reveal`). */
export const mockCalls: Array<{ cmd: string; args?: unknown }> = [];

let clipboard = "";
let pickedFolder: string | null = "C:/Musicas/Reverb";

export function resetMockMedia(): void {
  mockCalls.length = 0;
  clipboard = "";
  pickedFolder = "C:/Musicas/Reverb";
}

export function setMockClipboard(text: string): void {
  clipboard = text;
}

export function setMockPickedFolder(path: string | null): void {
  pickedFolder = path;
}

const KNOWN: Record<string, VideoInfo> = Object.fromEntries(
  [FX1_VIDEO, FX2_MUSIC, FX3_CLIP].map((v) => [v.id, v]),
);

function videoId(url: URL): string | null {
  if (url.hostname === "youtu.be") return url.pathname.split("/")[1] || null;
  const shorts = /^\/(?:shorts|live)\/([^/]+)/.exec(url.pathname);
  return url.searchParams.get("v") ?? shorts?.[1] ?? null;
}

export function mockUrlClassify(input: string): UrlKind {
  const hint = hintKind(input);
  const text = input.trim();
  if (hint === "search") return { kind: "search", query: text };
  if (hint === "empty" || hint === "unsupported") return { kind: "unsupported" };
  const url = new URL(/^www\./i.test(text) ? `https://${text}` : text);
  if (hint === "collection") return { kind: "collection", url: url.toString() };
  const id = videoId(url) ?? "";
  return {
    kind: "video",
    sourceId: id,
    url: `https://www.youtube.com/watch?v=${id}`,
    playlistHint: url.searchParams.has("list"),
  };
}

export function mockAnalyze(url: string): Analysis {
  const parsed = new URL(url);
  const id = videoId(parsed);
  if (id === null) return { type: "collection", info: FX4_ALBUM };
  if (id.startsWith("unavailable")) {
    throw { kind: "unavailable", message: "Video unavailable" };
  }
  const info = KNOWN[id] ?? {
    ...FX1_VIDEO,
    id,
    title: `Faixa ${id}`,
    webpageUrl: `https://www.youtube.com/watch?v=${id}`,
  };
  return { type: "video", info };
}

export function mockSearch(source: string, query: string): SearchResult[] {
  if (!query.trim()) return [];
  // O YouTube Music não informa duração (FX7); o YouTube comum sim.
  return source === "ytmusic"
    ? FX7_SEARCH.map((r) => ({ ...r, duration: null }))
    : FX7_SEARCH.map((r) => ({ ...r }));
}

export function mockPickFolder(): string | null {
  mockCalls.push({ cmd: "pick_folder" });
  return pickedFolder;
}

export function mockOpenOutputDir(): void {
  mockCalls.push({ cmd: "open_output_dir" });
}

export function mockClipboardReadText(): string {
  return clipboard;
}

export function mockLibraryReveal(path: string): void {
  mockCalls.push({ cmd: "library_reveal", args: { path } });
}

import type { Analysis } from "@/bindings/Analysis";
import type { SearchResult } from "@/bindings/SearchResult";
import type { UrlKind } from "@/bindings/UrlKind";
import type { VideoInfo } from "@/bindings/VideoInfo";
import { hintKind } from "@/lib/urlkind";
import { FX1_VIDEO, FX2_MUSIC, FX3_CLIP, FX4_ALBUM, FX7_SEARCH } from "./fixtures";
import { mockSettingsGet } from "./settings";
import { mockBus } from "./bus";

/** Chamadas das ações de sistema, para os testes conferirem (ex.: `library_reveal`). */
export const mockCalls: Array<{ cmd: string; args?: unknown }> = [];

let clipboard = "";
let lastClipboardUrl = "";
let pickedFolder: string | null = "C:/Musicas/Reverb";

export function resetMockMedia(): void {
  mockCalls.length = 0;
  clipboard = "";
  lastClipboardUrl = "";
  pickedFolder = "C:/Musicas/Reverb";
}

export function setMockClipboard(text: string): void {
  clipboard = text;
  if (!mockSettingsGet().clipboardWatch) {
    lastClipboardUrl = "";
    return;
  }
  const kind = mockUrlClassify(text);
  if ((kind.kind === "video" || kind.kind === "collection") && kind.url !== lastClipboardUrl) {
    lastClipboardUrl = kind.url;
    mockBus.emit("clipboard://url", { url: kind.url, visible: true });
  }
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
  if (!url.hostname.includes("youtube") && url.hostname !== "youtu.be") {
    return hint === "collection"
      ? { kind: "collection", url: url.toString() }
      : { kind: "video", url: url.toString(), sourceId: url.pathname, playlistHint: false };
  }
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
  if (parsed.hostname.includes("archive.org")) {
    const title = parsed.searchParams.get("file");
    if (!title)
      return {
        type: "collection",
        info: {
          id: "OpenGoldbergVariations",
          title: "The Open Goldberg Variations",
          channel: "Kimiko Ishizaka",
          thumbnail: null,
          entries: [
            {
              id: "OpenGoldbergVariations/Aria.flac",
              title: "Aria",
              duration: 181,
              url: "https://archive.org/details/OpenGoldbergVariations?file=Aria.flac",
            },
          ],
        },
      };
    return {
      type: "video",
      info: {
        ...FX2_MUSIC,
        id: `OpenGoldbergVariations/${title}`,
        title: "Aria",
        track: "Aria",
        artist: "Kimiko Ishizaka",
        album: "The Open Goldberg Variations",
        webpageUrl: url,
        extractorKey: "archive",
        audioFormats: [{ formatId: "original", acodec: "flac", ext: "flac", abr: null }],
        bestAudioAbr: null,
      },
    };
  }
  if (parsed.hostname.endsWith(".bandcamp.com")) {
    if (parsed.pathname.includes("paid")) throw { kind: "bandcamp_restricted" };
    return {
      type: "video",
      info: {
        ...FX2_MUSIC,
        id: "bandcamp-track",
        title: "Mellow Harmonics",
        track: "Mellow Harmonics",
        artist: "Sounds Like An Earful",
        webpageUrl: url,
        extractorKey: "bandcamp",
        bestAudioAbr: null,
      },
    };
  }
  if (parsed.hostname.includes("jamendo.com") || parsed.hostname.includes("soundcloud.com"))
    return {
      type: "video",
      info: {
        ...FX2_MUSIC,
        id: parsed.pathname,
        title: "Free track",
        webpageUrl: url,
        extractorKey: parsed.hostname.includes("jamendo") ? "jamendo" : "soundcloud",
        bestAudioAbr: null,
      },
    };
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
  if (source === "archive")
    return [
      {
        id: "OpenGoldbergVariations",
        title: "The Open Goldberg Variations",
        url: "https://archive.org/details/OpenGoldbergVariations",
        channel: "Kimiko Ishizaka",
        duration: null,
      },
    ];
  if (source === "jamendo") {
    if (!mockSettingsGet().secretsStatus.jamendo) throw { kind: "jamendo_key" };
    return [
      {
        id: "1",
        title: "Free Jamendo track",
        url: "https://www.jamendo.com/track/1",
        duration: 180,
        channel: "Artist",
      },
    ];
  }
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

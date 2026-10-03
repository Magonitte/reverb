import type { Candidate } from "@/bindings/Candidate";
import type { MetadataFields } from "@/bindings/MetadataFields";
import type { MetadataResult } from "@/bindings/MetadataResult";
import type { OfficialMatch } from "@/bindings/OfficialMatch";
import type { PreviewRequest } from "@/bindings/PreviewRequest";
import type { VideoInfo } from "@/bindings/VideoInfo";
import { FX2_MUSIC, FX3_CLIP } from "./fixtures";
import { mockAnalyze } from "./media";
import { mockSettingsGet } from "./settings";

/** Backend falso da identificação (F08): o clipe FX3 tem a faixa oficial FX2; FX1 não é música. */

/** Clipe ⇒ vídeo da faixa oficial correspondente. */
const OFFICIAL_OF: Record<string, VideoInfo> = { [FX3_CLIP.id]: FX2_MUSIC };

const NOISE = /\s*[([][^)\]]*(official|video|vídeo|audio|áudio|lyric|4k|hd|remaster)[^)\]]*[)\]]/gi;

function isMusic(video: VideoInfo): boolean {
  return (
    video.isOfficialTrack ||
    video.track !== null ||
    video.categories.some((c) => c.toLowerCase() === "music")
  );
}

/** "Artista - Título (Official Video)" ⇒ artista e título (a convenção do YouTube, esquerda = artista). */
function parseTitle(video: VideoInfo): { title: string; artist: string | null } {
  const raw = video.title.replace(NOISE, "").trim();
  const cut = raw.indexOf(" - ");
  if (cut > 0) return { artist: raw.slice(0, cut).trim(), title: raw.slice(cut + 3).trim() };
  return { artist: video.channel, title: raw };
}

function emptyFields(title: string, artist: string | null): MetadataFields {
  return {
    title,
    artist,
    artists: artist ? [artist] : [],
    album: null,
    albumArtist: null,
    year: null,
    genre: null,
    trackNo: null,
    trackTotal: null,
    discNo: null,
    coverUrl: null,
    mbRecordingId: null,
  };
}

/** Candidato do iTunes para "Never Gonna Give You Up" (dados reais gravados). */
const ITUNES_RICK: Candidate = {
  provider: "itunes",
  providerId: "1559885421",
  title: "Never Gonna Give You Up",
  artists: ["Rick Astley"],
  album: "Whenever You Need Somebody",
  albumArtist: null,
  year: 1987,
  genre: "Pop",
  trackNo: 1,
  trackTotal: 10,
  discNo: 1,
  durationS: 213.573,
  coverUrl: null,
  mbRecordingId: null,
  isrc: null,
};

export function mockFindOfficialVersion(
  video: VideoInfo,
  isrc?: string | null,
): OfficialMatch | null {
  if (video.isOfficialTrack) return null;
  const official = OFFICIAL_OF[video.id];
  if (!official) return null;
  return {
    videoId: official.id,
    url: `https://music.youtube.com/watch?v=${official.id}`,
    score: 0.97,
    title: official.track ?? official.title,
    artist: official.artist ?? "",
    album: official.album,
    isrc: isrc ?? null,
    via: isrc ? "isrc" : "text",
  };
}

/** Edição do usuário por cima dos campos (texto vazio = não informado). */
function applyOverride(fields: MetadataFields, edit: Record<string, unknown>): MetadataFields {
  const next = { ...fields };
  const text = (key: string): string | null => {
    const value = edit[key];
    return typeof value === "string" && value.trim() ? value.trim() : null;
  };
  const number = (key: string): number | null => {
    const value = edit[key];
    return typeof value === "number" ? value : null;
  };
  next.title = text("title") ?? next.title;
  const artist = text("artist");
  if (artist) {
    next.artist = artist;
    next.artists = [artist];
  }
  next.album = text("album") ?? next.album;
  next.albumArtist = text("albumArtist") ?? next.albumArtist;
  next.genre = text("genre") ?? next.genre;
  next.year = number("year") ?? next.year;
  next.trackNo = number("trackNo") ?? next.trackNo;
  next.trackTotal = number("trackTotal") ?? next.trackTotal;
  next.discNo = number("discNo") ?? next.discNo;
  return next;
}

/**
 * O que o pipeline faria com o vídeo. `useOfficial`: trocar o clipe pela faixa oficial (se houver).
 * Uma edição do usuário vence tudo (fonte `user`, confiança 1).
 */
export function mockMetadataFor(
  video: VideoInfo,
  options: { useOfficial?: boolean; override?: Record<string, unknown> | null } = {},
): MetadataResult {
  const official = mockFindOfficialVersion(video);
  const useOfficial = options.useOfficial ?? true;
  const music = isMusic(video);
  const swapped = !options.override && useOfficial && official ? OFFICIAL_OF[video.id]! : null;
  const source = swapped ?? video;

  let result: MetadataResult;
  if (
    source.extractorKey &&
    ["archive", "bandcamp", "jamendo", "soundcloud"].includes(source.extractorKey.toLowerCase())
  ) {
    result = {
      fields: {
        ...emptyFields(source.track ?? source.title, source.artist ?? source.uploader),
        album: source.album,
        year: source.releaseYear,
      },
      confidence: 1,
      source: source.extractorKey.toLowerCase(),
      bucket: "auto",
      candidates: [],
      contentType: "music",
      isrc: null,
      official: null,
    };
  } else if (!music) {
    result = {
      fields: emptyFields(video.title, video.channel),
      confidence: 0,
      source: "youtube",
      bucket: "none",
      candidates: [],
      contentType: "other",
      isrc: null,
      official: null,
    };
  } else if (source.isOfficialTrack) {
    result = {
      fields: {
        ...emptyFields(source.track ?? source.title, source.artist),
        album: source.album,
        year: source.releaseYear,
        genre: "Pop",
        trackNo: 1,
        trackTotal: 10,
        discNo: 1,
      },
      confidence: 1,
      source: "youtube_music",
      bucket: "auto",
      candidates: [],
      contentType: "music",
      isrc: null,
      official,
    };
  } else if (OFFICIAL_OF[video.id]) {
    // Clipe cuja faixa oficial o usuário não quer: o iTunes completa os campos.
    const parsed = parseTitle(video);
    result = {
      fields: {
        ...emptyFields(parsed.title, parsed.artist),
        album: ITUNES_RICK.album,
        year: ITUNES_RICK.year,
        genre: ITUNES_RICK.genre,
        trackNo: ITUNES_RICK.trackNo,
        trackTotal: ITUNES_RICK.trackTotal,
        discNo: ITUNES_RICK.discNo,
      },
      confidence: 0.96,
      source: "itunes",
      bucket: "auto",
      candidates: [{ score: 0.96, candidate: ITUNES_RICK }],
      contentType: "music",
      isrc: null,
      official,
    };
  } else {
    // Música sem correspondência confiável: base do título + um candidato para revisar.
    const parsed = parseTitle(video);
    result = {
      fields: emptyFields(parsed.title, parsed.artist),
      confidence: 0.7,
      source: "youtube",
      bucket: "review",
      candidates: [
        {
          score: 0.7,
          candidate: {
            ...ITUNES_RICK,
            providerId: "0",
            title: parsed.title,
            artists: parsed.artist ? [parsed.artist] : [],
          },
        },
      ],
      contentType: "music",
      isrc: null,
      official: null,
    };
  }

  if (options.override) {
    return {
      ...result,
      fields: applyOverride(result.fields, options.override),
      confidence: 1,
      source: "user",
      bucket: "auto",
      candidates: [],
    };
  }
  return result;
}

export function mockMetadataPreview(request: PreviewRequest): MetadataResult {
  let video = request.video;
  if (!video) {
    const analysis = mockAnalyze(request.url);
    if (analysis.type !== "video") {
      throw {
        kind: "unavailable",
        message: "a pré-visualização de metadados vale para um vídeo, não para uma coleção",
      };
    }
    video = analysis.info;
  }
  const useOfficial = request.useOfficial ?? mockSettingsGet().preferOfficialAudio;
  return mockMetadataFor(video, { useOfficial });
}

export function mockMetadataSearch(query: string): Candidate[] {
  if (!query.trim()) return [];
  return /rick|never gonna/i.test(query)
    ? [
        {
          ...ITUNES_RICK,
          coverUrl: new URL("/mock-cover.svg", window.location.origin).href,
        },
      ]
    : [];
}

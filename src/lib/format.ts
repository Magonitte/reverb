import type { VideoInfo } from "@/bindings/VideoInfo";

/** `214` ⇒ `3:34`; `3725` ⇒ `1:02:05`. */
export function formatDuration(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || !Number.isFinite(seconds)) return "";
  const total = Math.max(0, Math.round(seconds));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const ss = String(s).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
}

/** Bytes por segundo ⇒ `2,3 MB/s`. */
export function formatSpeed(bps: number | null | undefined): string {
  if (bps === null || bps === undefined || bps <= 0) return "";
  if (bps >= 1_000_000) return `${(bps / 1_000_000).toFixed(1)} MB/s`;
  return `${Math.max(1, Math.round(bps / 1000))} kB/s`;
}

const CODEC_NAMES: Array<[RegExp, string]> = [
  [/^opus/i, "Opus"],
  [/^(mp4a|aac)/i, "AAC"],
  [/^mp3|^mp4a\.40\.34/i, "MP3"],
  [/^vorbis/i, "Vorbis"],
  [/^flac/i, "FLAC"],
];

export function codecLabel(acodec: string | null | undefined): string {
  if (!acodec) return "";
  return CODEC_NAMES.find(([re]) => re.test(acodec))?.[1] ?? acodec;
}

/** Melhor formato de áudio da fonte (selo "Fonte: Opus 129 kbps"). */
export function sourceQuality(info: VideoInfo): { codec: string; abr: number } | null {
  const best = info.audioFormats
    .filter((f) => f.abr !== null && f.abr !== undefined)
    .sort((a, b) => (b.abr ?? 0) - (a.abr ?? 0))[0];
  if (!best || best.abr === null) return null;
  return { codec: codecLabel(best.acodec), abr: Math.round(best.abr) };
}

/** URL de miniatura de um vídeo do YouTube a partir do id (resultados de busca não trazem uma). */
export function thumbnailFor(id: string): string {
  return `https://i.ytimg.com/vi/${id}/mqdefault.jpg`;
}

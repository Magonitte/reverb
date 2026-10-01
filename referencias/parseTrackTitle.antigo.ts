/** Separa título e artista a partir do título bruto do YouTube (ex.: "Song — Artist"). */
export function parseTrackTitle(
  rawTitle: string,
  fallbackArtist?: string,
): { title: string; artist: string } {
  const match = rawTitle.match(/^(.+?)\s+[–—-]\s+(.+)$/);
  if (match) {
    return { title: match[1].trim(), artist: match[2].trim() };
  }
  return { title: rawTitle.trim(), artist: fallbackArtist?.trim() || "—" };
}

export function importUrl(input: string): { provider: string; kind: string; id: string } | null {
  try {
    const u = new URL(input.trim());
    if (!["https:", "http:"].includes(u.protocol) || u.username || u.password) return null;
    if (u.hostname === "link.deezer.com")
      return { provider: "deezer", kind: "short", id: u.pathname };
    const provider = ["deezer.com", "www.deezer.com"].includes(u.hostname)
      ? "deezer"
      : u.hostname === "open.spotify.com"
        ? "spotify"
        : null;
    if (!provider) return null;
    const parts = u.pathname.split("/").filter(Boolean);
    if (parts[0]?.length === 2 || parts[0]?.startsWith("intl-")) parts.shift();
    if (parts.length !== 2 || !["playlist", "album", "track", "artist"].includes(parts[0]))
      return null;
    if (!(provider === "deezer" ? /^\d+$/ : /^[a-zA-Z0-9]+$/).test(parts[1])) return null;
    return { provider, kind: parts[0], id: parts[1] };
  } catch {
    return null;
  }
}

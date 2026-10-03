/** Dica visual da barra de comando. A decisão final é do backend (`url_classify`). */
export type KindHint = "empty" | "video" | "collection" | "search" | "unsupported";

const YOUTUBE_HOSTS = /^(www\.|m\.|music\.)?(youtube\.com|youtu\.be)$/i;

export function hintKind(input: string): KindHint {
  const text = input.trim();
  if (!text) return "empty";
  if (!/^(https?:\/\/|www\.)/i.test(text)) return "search";
  let url: URL;
  try {
    url = new URL(/^www\./i.test(text) ? `https://${text}` : text);
  } catch {
    return "unsupported";
  }
  if (/^(www\.)?archive\.org$/i.test(url.hostname) && /^\/details\/[^/]+\/?$/.test(url.pathname))
    return url.searchParams.has("file") ? "video" : "collection";
  if (/^(www\.)?soundcloud\.com$/i.test(url.hostname) && /^\/[^/]+\/[^/]+\/?$/.test(url.pathname))
    return "video";
  if (
    /^(www\.)?soundcloud\.com$/i.test(url.hostname) &&
    /^\/[^/]+\/sets\/[^/]+\/?$/.test(url.pathname)
  )
    return "collection";
  if (url.hostname.endsWith(".bandcamp.com") && /^\/(album|track)\/[^/]+\/?$/.test(url.pathname))
    return url.pathname.startsWith("/album/") ? "collection" : "video";
  if (
    /^(www\.)?jamendo\.com$/i.test(url.hostname) &&
    /^\/track\/\d+(?:\/[^/]*)?\/?$/.test(url.pathname)
  )
    return "video";
  if (!YOUTUBE_HOSTS.test(url.hostname)) return "unsupported";
  const path = url.pathname;
  const list = url.searchParams.get("list");
  if (path.startsWith("/playlist") || path.startsWith("/channel/") || path.startsWith("/@")) {
    return "collection";
  }
  if (url.searchParams.get("v") || url.hostname === "youtu.be" || path.startsWith("/shorts/")) {
    return "video";
  }
  return list ? "collection" : "unsupported";
}

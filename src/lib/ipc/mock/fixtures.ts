import type { SearchResult } from "@/bindings/SearchResult";
import type { CollectionInfo } from "@/bindings/CollectionInfo";
import type { VideoInfo } from "@/bindings/VideoInfo";

/** FX1–FX4 (`tests/fixtures/ytdlp/`) reduzidos ao que a UI usa. Dados reais gravados do yt-dlp. */

const BASE: VideoInfo = {
  id: "",
  title: "",
  duration: null,
  channel: null,
  uploader: null,
  thumbnail: null,
  thumbnails: [],
  categories: [],
  track: null,
  artist: null,
  artists: [],
  creators: [],
  album: null,
  releaseYear: null,
  releaseDate: null,
  chapters: [],
  audioFormats: [],
  bestAudioAbr: null,
  isOfficialTrack: false,
  webpageUrl: null,
  extractorKey: "Youtube",
};

/** FX1: vídeo comum, sem metadados de música. */
export const FX1_VIDEO: VideoInfo = {
  ...BASE,
  id: "jNQXAC9IVRw",
  title: "Me at the zoo",
  duration: 19,
  channel: "jawed",
  uploader: "jawed",
  thumbnail: "https://i.ytimg.com/vi/jNQXAC9IVRw/hqdefault.jpg",
  categories: ["People & Blogs"],
  audioFormats: [{ formatId: "251", acodec: "opus", abr: 129, ext: "webm" }],
  bestAudioAbr: 129,
  webpageUrl: "https://www.youtube.com/watch?v=jNQXAC9IVRw",
};

/** FX2: faixa oficial do YouTube Music (com `track`/`artist`). */
export const FX2_MUSIC: VideoInfo = {
  ...BASE,
  id: "lYBUbBu4W08",
  title: "Never Gonna Give You Up",
  duration: 214,
  channel: "Rick Astley",
  artist: "Rick Astley",
  artists: ["Rick Astley"],
  track: "Never Gonna Give You Up",
  album: "Whenever You Need Somebody",
  releaseYear: 1987,
  thumbnail: "https://i.ytimg.com/vi_webp/lYBUbBu4W08/maxresdefault.webp",
  categories: ["Music"],
  audioFormats: [{ formatId: "251", acodec: "opus", abr: 129, ext: "webm" }],
  bestAudioAbr: 129,
  isOfficialTrack: true,
  webpageUrl: "https://music.youtube.com/watch?v=lYBUbBu4W08",
  extractorKey: "Youtube",
};

/** FX3: clipe oficial (título "Artista - Faixa (Official Video)"). */
export const FX3_CLIP: VideoInfo = {
  ...BASE,
  id: "dQw4w9WgXcQ",
  title: "Rick Astley - Never Gonna Give You Up (Official Video) (4K Remaster)",
  duration: 213,
  channel: "Rick Astley",
  uploader: "Rick Astley",
  thumbnail: "https://i.ytimg.com/vi_webp/dQw4w9WgXcQ/maxresdefault.webp",
  categories: ["Music"],
  audioFormats: [{ formatId: "251", acodec: "opus", abr: 129, ext: "webm" }],
  bestAudioAbr: 129,
  webpageUrl: "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
};

/** FX4: álbum (10 faixas; as 3 primeiras são reais, as demais têm ids sintéticos). */
export const FX4_ALBUM: CollectionInfo = {
  id: "OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE",
  title: "Album - Whenever You Need Somebody",
  channel: "Rick Astley",
  entries: [
    { id: "lYBUbBu4W08", title: "Never Gonna Give You Up", duration: 214, url: null },
    { id: "raBobo3GZYA", title: "Whenever You Need Somebody", duration: 234, url: null },
    { id: "i_Q88T1HI_w", title: "Together Forever", duration: 206, url: null },
    ...[
      ["It Would Take a Strong Strong Man", 256],
      ["The Love Has Gone", 213],
      ["Don't Say Goodbye", 240],
      ["Slipping Away", 233],
      ["No More Looking for Love", 270],
      ["You Move Me", 225],
      ["When I Fall in Love", 197],
    ].map(([title, duration], i) => ({
      id: `fx4-track-${i + 4}`,
      title: title as string,
      duration: duration as number,
      url: null,
    })),
  ],
};

export const FX_VIDEOS: VideoInfo[] = [FX1_VIDEO, FX2_MUSIC, FX3_CLIP];

/** FX7: busca "rick astley never gonna give you up" (primeiro resultado `lYBUbBu4W08`). */
export const FX7_SEARCH: SearchResult[] = [
  {
    id: "lYBUbBu4W08",
    title: "Never Gonna Give You Up",
    url: "https://music.youtube.com/watch?v=lYBUbBu4W08",
    duration: 214,
    channel: "Rick Astley",
  },
  {
    id: "dQw4w9WgXcQ",
    title: "Rick Astley - Never Gonna Give You Up (Official Video)",
    url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    duration: 213,
    channel: "Rick Astley",
  },
  {
    id: "jNQXAC9IVRw",
    title: "Me at the zoo",
    url: "https://www.youtube.com/watch?v=jNQXAC9IVRw",
    duration: 19,
    channel: "jawed",
  },
];

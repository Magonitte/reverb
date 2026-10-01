// Grava as fixtures JSON do yt-dlp (F03/T12) em tests/fixtures/ytdlp/, sem URLs assinadas.
// Uso: node scripts/record-fixtures.mjs   (precisa de rede e de `npm run test:prepare`)
import { spawnSync } from "node:child_process";
import { mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const toolsDir = process.env.REVERB_TEST_TOOLS_DIR ?? ".test-tools";
const only = (dir) => join(dir, readdirSync(dir)[0]);
const ytdlp = join(only(join(toolsDir, "ytdlp")), process.platform === "win32" ? "yt-dlp.exe" : "yt-dlp_linux");
const deno = join(only(join(toolsDir, "deno")), process.platform === "win32" ? "deno.exe" : "deno");
const ffmpegDir = only(join(toolsDir, "ffmpeg"));
const out = "tests/fixtures/ytdlp";
mkdirSync(out, { recursive: true });

const base = [
  "--ignore-config",
  "--color",
  "never",
  "--js-runtimes",
  `deno:${deno}`,
  "--ffmpeg-location",
  ffmpegDir,
];

function run(args) {
  const result = spawnSync(ytdlp, [...base, ...args], {
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
    env: { ...process.env, PYTHONIOENCODING: "utf-8", PYTHONUTF8: "1" },
  });
  return result;
}

// Remove tudo que carrega URL assinada: formatos, legendas e downloads solicitados.
function scrub(info) {
  for (const format of info.formats ?? []) {
    delete format.url;
    delete format.manifest_url;
    delete format.fragments;
    delete format.http_headers;
  }
  for (const key of ["requested_formats", "requested_downloads", "http_headers"]) delete info[key];
  info.automatic_captions = {};
  info.subtitles = {};
  return info;
}

const jobs = [
  ["fx1-video.json", ["-J", "--no-playlist", "--", "https://www.youtube.com/watch?v=jNQXAC9IVRw"], true],
  ["fx2-music.json", ["-J", "--no-playlist", "--", "https://music.youtube.com/watch?v=lYBUbBu4W08"], true],
  ["fx3-clip.json", ["-J", "--no-playlist", "--", "https://www.youtube.com/watch?v=dQw4w9WgXcQ"], true],
  [
    "fx4-album.json",
    ["-J", "--flat-playlist", "--", "https://www.youtube.com/playlist?list=OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE"],
    false,
  ],
];

for (const [name, args, isVideo] of jobs) {
  const result = run(args);
  if (result.status !== 0) {
    console.error(`${name}: falhou\n${result.stderr}`);
    process.exit(1);
  }
  let json = JSON.parse(result.stdout);
  if (isVideo) json = scrub(json);
  writeFileSync(join(out, name), JSON.stringify(json, null, 1) + "\n");
  console.log(`gravado ${name}`);
}

// stderr real de "Video unavailable" (ID inexistente).
const bad = run(["-J", "--no-playlist", "--", "https://www.youtube.com/watch?v=aaaaaaaaaaa"]);
if (bad.status === 0) {
  console.error("esperava falha para o ID inexistente");
  process.exit(1);
}
writeFileSync(join(out, "stderr-unavailable.txt"), bad.stderr.replace(/\r\n/g, "\n"));
console.log("gravado stderr-unavailable.txt");

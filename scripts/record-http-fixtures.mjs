// Grava as respostas HTTP reais dos provedores de metadados (F08/T11) em tests/fixtures/http/.
// Consultas de FX3 (clipe do Rick Astley, mesma música de FX2) e de FX1 ("Me at the zoo").
// Uso: node scripts/record-http-fixtures.mjs   (precisa de rede)
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const out = "tests/fixtures/http";
mkdirSync(out, { recursive: true });

const UA = "Reverb/0.1.1 (+https://github.com/Magonitte/reverb)";
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

// Previews de áudio trazem URLs assinadas (tokens que expiram): fora das fixtures.
function scrub(value) {
  if (Array.isArray(value)) return value.map(scrub);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value)
        .filter(([key]) => key !== "preview" && key !== "previewUrl")
        .map(([key, inner]) => [key, scrub(inner)]),
    );
  }
  return value;
}

async function get(name, url) {
  const response = await fetch(url, { headers: { "User-Agent": UA } });
  if (!response.ok) {
    console.error(`${name}: HTTP ${response.status} em ${url}`);
    process.exit(1);
  }
  const text = await response.text();
  const json = scrub(JSON.parse(text));
  writeFileSync(join(out, name), `${JSON.stringify(json, null, 1)}\n`);
  console.log(`gravado ${name}`);
  return json;
}

const q = (text) => encodeURIComponent(text);

for (const [tag, artist, title] of [
  ["fx3", "Rick Astley", "Never Gonna Give You Up"],
  ["fx1", "jawed", "Me at the zoo"],
]) {
  const deezer = await get(`deezer-search-${tag}.json`, `https://api.deezer.com/search?q=${q(`${artist} ${title}`)}&limit=5`);
  if (tag === "fx3") {
    const track = deezer.data?.[0];
    if (!track) throw new Error("Deezer sem resultados para FX3");
    const detail = await get(`deezer-track-${track.id}.json`, `https://api.deezer.com/track/${track.id}`);
    await get(`deezer-album-${detail.album.id}.json`, `https://api.deezer.com/album/${detail.album.id}`);
  }
  await sleep(400);
  await get(`itunes-search-${tag}.json`, `https://itunes.apple.com/search?term=${q(`${artist} ${title}`)}&entity=song&limit=5`);
  await sleep(1100);
  await get(
    `musicbrainz-recording-${tag}.json`,
    `https://musicbrainz.org/ws/2/recording?query=${q(`recording:"${title}" AND artist:"${artist}"`)}&fmt=json&limit=5`,
  );
  await sleep(1100);
}

// Busca reversa por ISRC (E1): a gravação original de 1987.
await get("deezer-isrc-GBARL9300135.json", "https://api.deezer.com/track/isrc:GBARL9300135");

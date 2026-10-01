import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const exe = join("target", "debug", process.platform === "win32" ? "reverb.exe" : "reverb");
if (!existsSync(exe)) {
  console.error(`Executável não encontrado: ${exe} (rode: cargo build -p reverb)`);
  process.exit(1);
}

const tmp = mkdtempSync(join(tmpdir(), "reverb-selftest-"));
const out = join(tmp, "reverb-selftest.json");
try {
  const r = spawnSync(exe, ["--headless-selftest", out], { timeout: 60_000 });
  if (r.status !== 0) {
    console.error(`Autoteste headless saiu com código ${r.status}`);
    process.exit(1);
  }
  const json = JSON.parse(readFileSync(out, "utf8"));
  if (json.ok !== true || !json.version || !json.dataDir) {
    console.error(`JSON inesperado: ${JSON.stringify(json)}`);
    process.exit(1);
  }
  console.log(`Autoteste headless OK: ${JSON.stringify(json)}`);
} finally {
  rmSync(tmp, { recursive: true, force: true });
}

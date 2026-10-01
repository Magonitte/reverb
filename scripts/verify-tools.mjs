// Confirma que as ferramentas instaladas por `test:prepare` executam de verdade
// (usado no CI Linux — F02/T13): yt-dlp, deno, ffmpeg, ffprobe e fpcalc.
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";

const toolsDir = process.env.REVERB_TEST_TOOLS_DIR ?? ".test-tools";

const status = spawnSync(
  "cargo",
  ["run", "-q", "-p", "reverb-cli", "--release", "--", "tools", "status", "--tools-dir", toolsDir],
  { encoding: "utf8", env: process.env },
);
if (status.status !== 0) {
  console.error(status.stderr);
  process.exit(status.status ?? 1);
}
const tools = JSON.parse(status.stdout);

const checks = [];
for (const [id, args] of [
  ["ytdlp", ["--version"]],
  ["deno", ["--version"]],
  ["ffmpeg", ["-version"]],
  ["fpcalc", ["-version"]],
]) {
  const tool = tools.find((t) => t.tool === id);
  if (!tool?.installed) {
    checks.push({ name: id, ok: false, detail: "não instalado" });
    continue;
  }
  checks.push({ name: id, ok: true, path: tool.path, args });
  if (id === "ffmpeg") {
    checks.push({
      name: "ffprobe",
      ok: true,
      path: join(dirname(tool.path), process.platform === "win32" ? "ffprobe.exe" : "ffprobe"),
      args,
    });
  }
}

let failed = false;
for (const check of checks) {
  if (!check.ok) {
    console.error(`FALHA ${check.name}: ${check.detail}`);
    failed = true;
    continue;
  }
  const run = spawnSync(check.path, check.args, { encoding: "utf8" });
  const output = `${run.stdout}${run.stderr}`.split("\n")[0].trim();
  if (run.status === 0) {
    console.log(`OK    ${check.name}: ${output}`);
  } else {
    console.error(`FALHA ${check.name}: código ${run.status} ${run.error ?? ""} ${output}`);
    failed = true;
  }
}
process.exit(failed ? 1 : 0);

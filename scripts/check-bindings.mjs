import { spawnSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const dir = "src/bindings";
const snapshot = (root) =>
  Object.fromEntries(
    (existsSync(root) ? readdirSync(root) : [])
      .sort()
      .map((f) => [f, readFileSync(join(root, f), "utf8")]),
  );

mkdirSync(dir, { recursive: true });
const backup = mkdtempSync(join(tmpdir(), "reverb-bindings-"));
cpSync(dir, backup, { recursive: true });
const before = snapshot(backup);

const r = spawnSync("npm run bindings", { stdio: "inherit", shell: true });
const after = snapshot(dir);
rmSync(backup, { recursive: true, force: true });

if (r.status !== 0) process.exit(r.status ?? 1);
const names = new Set([...Object.keys(before), ...Object.keys(after)]);
const diff = [...names].filter((n) => before[n] !== after[n]);
if (diff.length > 0) {
  console.error(`Bindings TS desatualizados (regenerados agora; faça commit): ${diff.join(", ")}`);
  process.exit(1);
}
console.log("check:bindings: bindings atualizados");

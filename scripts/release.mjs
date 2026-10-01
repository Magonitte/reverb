// Release local: árvore limpa, verify, bump, commit e tag.
// O push (e portanto o workflow) só acontece com --push, depois da confirmação.
import { spawnSync } from "node:child_process";

const args = process.argv.slice(2);
const push = args.includes("--push");
const version = args.find((arg) => !arg.startsWith("--"));
if (!/^\d+\.\d+\.\d+$/.test(version ?? "")) {
  console.error("uso: node scripts/release.mjs <x.y.z> [--push]");
  process.exit(1);
}

function run(command, commandArgs) {
  const result = spawnSync(command, commandArgs, { stdio: "inherit" });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

function capture(command, commandArgs) {
  const result = spawnSync(command, commandArgs, { encoding: "utf8" });
  if (result.status !== 0) process.exit(result.status ?? 1);
  return result.stdout;
}

const dirty = capture("git", ["status", "--porcelain"]).trim();
if (dirty) {
  console.error("árvore suja; commit ou descarte as mudanças antes da release");
  process.exit(1);
}

run("npm", ["run", "verify"]);
run("node", ["scripts/bump-version.mjs", version]);
run("git", ["add", "package.json", "package-lock.json", "Cargo.toml", "Cargo.lock", "CHANGELOG.md"]);
run("git", ["commit", "-m", `chore(release): v${version}`]);
run("git", ["tag", `v${version}`]);

if (push) {
  run("git", ["push", "origin", "HEAD", "--follow-tags"]);
} else {
  console.log(`tag v${version} criada localmente. Push só com --push, depois da confirmação.`);
}

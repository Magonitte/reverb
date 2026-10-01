import { spawnSync } from "node:child_process";

const steps = [
  ["typecheck", "npm run typecheck"],
  ["lint", "npm run lint"],
  ["check:i18n", "npm run check:i18n"],
  ["check:ipc", "npm run check:ipc"],
  ["test (vitest)", "npm test"],
  ["build", "npm run build"],
  ["cargo fmt", "cargo fmt --all --check"],
  ["cargo clippy", "cargo clippy --workspace --all-targets -- -D warnings"],
  ["cargo test", "cargo test --workspace"],
  ["check:bindings", "npm run check:bindings"],
  ["scan:secrets", "npm run scan:secrets"],
];

const results = [];
const started = Date.now();
for (const [name, command] of steps) {
  const t0 = Date.now();
  console.log(`\n=== ${name}: ${command}`);
  const r = spawnSync(command, { stdio: "inherit", shell: true });
  const secs = ((Date.now() - t0) / 1000).toFixed(1);
  results.push({ name, ok: r.status === 0, secs });
  if (r.status !== 0) break;
}

console.log("\n--- Resumo do verify ---");
for (const r of results) console.log(`${r.ok ? "OK   " : "FALHA"} ${r.name} (${r.secs}s)`);
console.log(`Total: ${((Date.now() - started) / 1000).toFixed(1)}s`);
process.exit(results.every((r) => r.ok) && results.length === steps.length ? 0 : 1);

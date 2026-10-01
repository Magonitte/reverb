import { spawnSync } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";

const env = { ...process.env, REVERB_NET_TESTS: "1" };
const run = (command) => spawnSync(command, { stdio: "inherit", shell: true, env }).status === 0;

let ok = run("cargo test --workspace -- --ignored");

const hasE2e =
  existsSync("tests/e2e") && readdirSync("tests/e2e").some((f) => f.endsWith(".spec.ts"));
if (ok && hasE2e) ok = run("npx playwright test --grep @network --pass-with-no-tests");

process.exit(ok ? 0 : 1);

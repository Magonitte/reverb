// Sincroniza a versão do app: package.json e o workspace Cargo (arquitetura §17).
import { readFileSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(version ?? "")) {
  console.error("uso: node scripts/bump-version.mjs <x.y.z>");
  process.exit(1);
}

const pkgPath = "package.json";
const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
pkg.version = version;
writeFileSync(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`);

const cargoPath = "Cargo.toml";
const cargo = readFileSync(cargoPath, "utf8");
const next = cargo.replace(
  /(\[workspace\.package\][\s\S]*?^version = ")[^"]+(")/m,
  `$1${version}$2`,
);
if (next === cargo || !next.includes(`version = "${version}"`)) {
  console.error("não achei version em [workspace.package] no Cargo.toml");
  process.exit(1);
}
writeFileSync(cargoPath, next);

const update = spawnSync("cargo", ["update", "-w"], { stdio: "inherit" });
if (update.status !== 0) process.exit(update.status ?? 1);
console.log(`versão ${version}`);

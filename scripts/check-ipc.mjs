// Garante que Rust (generate_handler!), COMMANDS/wrappers de api.ts e o backend mock têm o mesmo conjunto de comandos.
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

/** Nomes dentro de `tauri::generate_handler![ ... ]` (último segmento do caminho). */
export function extractRustCommands(source) {
  const match = /generate_handler!\s*\[([\s\S]*?)\]/.exec(source);
  if (!match) return [];
  return match[1]
    .split(",")
    .map((item) => item.replace(/\/\/.*$/gm, "").trim())
    .filter(Boolean)
    .map((path) => path.split("::").pop());
}

/** Literais do array `COMMANDS` em api.ts. */
export function extractCommandsList(source) {
  const match = /export const COMMANDS\s*=\s*\[([\s\S]*?)\]/.exec(source);
  if (!match) return [];
  return [...match[1].matchAll(/["']([a-z0-9_]+)["']/g)].map((m) => m[1]);
}

/** Primeiro argumento de cada `call<...>("nome"` nos wrappers de `api`. */
export function extractWrapperCommands(source) {
  return [...source.matchAll(/\bcall<[^>]*(?:<[^>]*>)?[^>]*>\(\s*["']([a-z0-9_]+)["']/g)].map(
    (m) => m[1],
  );
}

/** Chaves do objeto `handlers` do mock. */
export function extractMockCommands(source) {
  const match = /const handlers[^\n]*=\s*\{\n([\s\S]*?)\n\};/.exec(source);
  if (!match) return [];
  return [...match[1].matchAll(/^\s{2}([a-z0-9_]+):/gm)].map((m) => m[1]);
}

/** Compara os conjuntos; devolve a lista de problemas (vazia = ok). */
export function compareCommandSets(sets) {
  const names = Object.keys(sets);
  const all = new Set(names.flatMap((n) => sets[n]));
  const problems = [];
  for (const [name, list] of Object.entries(sets)) {
    const dup = list.filter((c, i) => list.indexOf(c) !== i);
    for (const c of new Set(dup)) problems.push(`comando duplicado em ${name}: ${c}`);
  }
  for (const command of [...all].sort()) {
    const missing = names.filter((n) => !sets[n].includes(command));
    if (missing.length > 0) problems.push(`comando "${command}" falta em: ${missing.join(", ")}`);
  }
  return problems;
}

function main() {
  const read = (path) => readFileSync(path, "utf8");
  const api = read("src/lib/ipc/api.ts");
  const sets = {
    "Rust (generate_handler!)": extractRustCommands(read("src-tauri/src/lib.rs")),
    "COMMANDS (api.ts)": extractCommandsList(api),
    "wrappers (api.ts)": extractWrapperCommands(api),
    "mock (handlers)": extractMockCommands(read("src/lib/ipc/mock/index.ts")),
  };
  for (const [name, list] of Object.entries(sets)) {
    if (list.length === 0) {
      console.error(`check:ipc: não achei nenhum comando em ${name}`);
      process.exit(1);
    }
  }
  const problems = compareCommandSets(sets);
  if (problems.length > 0) {
    console.error(problems.join("\n"));
    process.exit(1);
  }
  console.log(`check:ipc: ${sets["COMMANDS (api.ts)"].length} comandos iguais nos 4 lugares`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();

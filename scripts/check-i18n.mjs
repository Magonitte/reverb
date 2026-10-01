import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export function flatten(obj, prefix = "") {
  const out = {};
  for (const [key, value] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (value !== null && typeof value === "object") Object.assign(out, flatten(value, path));
    else out[path] = value;
  }
  return out;
}

/** Compara dois dicionários de tradução; devolve a lista de problemas (vazia = ok). */
export function compareLocales(a, b, nameA = "pt-BR", nameB = "en") {
  const fa = flatten(a);
  const fb = flatten(b);
  const problems = [];
  for (const key of Object.keys(fa)) {
    if (!(key in fb)) problems.push(`chave "${key}" existe em ${nameA} mas falta em ${nameB}`);
  }
  for (const key of Object.keys(fb)) {
    if (!(key in fa)) problems.push(`chave "${key}" existe em ${nameB} mas falta em ${nameA}`);
  }
  for (const [name, flat] of [
    [nameA, fa],
    [nameB, fb],
  ]) {
    for (const [key, value] of Object.entries(flat)) {
      if (typeof value !== "string" || value.trim() === "")
        problems.push(`valor vazio em ${name}: "${key}"`);
    }
  }
  return problems;
}

function main() {
  const load = (lang) => JSON.parse(readFileSync(`src/locales/${lang}/translation.json`, "utf8"));
  const problems = compareLocales(load("pt-BR"), load("en"));
  if (problems.length > 0) {
    console.error(problems.join("\n"));
    process.exit(1);
  }
  console.log("i18n: pt-BR e en consistentes");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();

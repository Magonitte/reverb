import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export const PATTERNS = [
  { name: "chave privada", re: /-----BEGIN .*PRIVATE KEY-----/ },
  { name: "access_token", re: /"access_token"\s*:\s*"[A-Za-z0-9_-]{20,}/ },
  { name: "client_secret", re: /client_secret\s*[=:]\s*["'][^"']{8,}/ },
  { name: "chave de assinatura do Tauri", re: /TAURI_SIGNING_PRIVATE_KEY=/ },
];

/** Devolve as linhas (1-based) e o nome do padrão que casaram no conteúdo. */
export function scanContent(content) {
  const hits = [];
  content.split(/\r?\n/).forEach((line, index) => {
    for (const { name, re } of PATTERNS) {
      if (re.test(line)) hits.push({ line: index + 1, pattern: name });
    }
  });
  return hits;
}

function listFiles() {
  const out = execFileSync("git", ["ls-files", "--cached", "--others", "--exclude-standard"], {
    encoding: "utf8",
  });
  return out
    .split("\n")
    .map((f) => f.trim())
    .filter((f) => f && !f.startsWith("plano/") && !f.startsWith("scripts/"));
}

function main() {
  let failed = false;
  for (const file of listFiles()) {
    let content;
    try {
      content = readFileSync(file, "utf8");
    } catch {
      continue;
    }
    for (const hit of scanContent(content)) {
      console.error(`${file}:${hit.line}: possível segredo (${hit.pattern})`);
      failed = true;
    }
  }
  if (failed) process.exit(1);
  console.log("scan:secrets: nenhum segredo encontrado");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();

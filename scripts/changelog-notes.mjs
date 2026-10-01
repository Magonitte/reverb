// Imprime a seção do CHANGELOG da versão atual do package.json (corpo da release).
import { readFileSync } from "node:fs";

const version = JSON.parse(readFileSync("package.json", "utf8")).version;
const changelog = readFileSync("CHANGELOG.md", "utf8");
const header = `## ${version}`;
const start = changelog.indexOf(header);
if (start < 0) {
  console.error(`CHANGELOG.md não tem a seção ${header}`);
  process.exit(1);
}
const rest = changelog.slice(start + header.length).replace(/^\r?\n/, "");
const next = rest.search(/^## /m);
const body = (next < 0 ? rest : rest.slice(0, next)).trim();
if (!body) {
  console.error(`seção ${header} está vazia`);
  process.exit(1);
}
process.stdout.write(body);

// Instala as ferramentas reais (yt-dlp, Deno, FFmpeg, fpcalc) em `.test-tools/` para os testes
// que precisam delas. Idempotente: o CLI pula o que já está instalado e atualizado.
import { spawnSync } from "node:child_process";

const toolsDir = process.env.REVERB_TEST_TOOLS_DIR ?? ".test-tools";

// Argumentos sempre como lista (caminhos podem ter espaços); o CLI é compilado aqui porque
// `cargo test` não o compila antes de rodar este script.
const result = spawnSync(
  "cargo",
  [
    "run",
    "-p",
    "reverb-cli",
    "--release",
    "--",
    "tools",
    "install",
    "--all",
    "--tools-dir",
    toolsDir,
  ],
  { stdio: "inherit", env: process.env },
);

if (result.status !== 0) {
  console.error("test:prepare: falha ao instalar as ferramentas de teste");
  process.exit(result.status ?? 1);
}
console.log(`test:prepare: ferramentas prontas em ${toolsDir}`);

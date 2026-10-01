# F02 — Gerenciador de ferramentas (yt-dlp, Deno, FFmpeg, fpcalc)

**Objetivo:** o app baixa, verifica (hash), instala, atualiza, reverte e informa a versão das
ferramentas externas sozinho, no Windows e no Linux. Esta é a peça que impede a quebra que o
projeto antigo sofreu.

**Pré-requisito:** F01. **Arquitetura:** §16, §8 (runtime JS), §19.

## Tarefas

1. `reverb-core::tools::github`: cliente da API de releases (`latest` por repo), com `User-Agent`,
   `Accept`, `GITHUB_TOKEN` opcional, cache em memória + `kv` por 1 h, base URL injetável.
2. `ToolSpec` declarativo para `ytdlp` (canais stable/nightly), `deno`, `ffmpeg` (inclui
   `ffprobe` no mesmo pacote), `fpcalc`: repo, regex do asset por (SO, arquitetura), tipo de
   pacote (exe solto, zip, tar.xz, tar.gz), caminho do binário dentro do pacote (ex.: FFmpeg
   `ffmpeg-master-latest-win64-gpl/bin/ffmpeg.exe` — localizar por busca do nome do binário dentro
   da extração, não por caminho fixo), fonte do checksum, comando e regex de versão.
3. Parsers de checksum (3 formatos de §16) e de versão; comparação: yt-dlp por data
   (`YYYY.MM.DD[.HHMMSS]`), Deno/fpcalc por `semver`, FFmpeg por `updated_at`.
4. Instalação (§16): staging, progresso (`tools://progress {tool, phase: "downloading"|"verifying"|"extracting"|"testing"|"waiting_jobs", percent}`),
   verificação de hash obrigatória quando a fonte tem checksum (divergência ⇒ erro, nada é
   trocado), extração (zip via crate; tar via `tar` do sistema no Linux), `chmod 755`, teste de
   fumaça, troca atômica do `current` no `tools/manifest.json`, limpeza de versões antigas.
5. `ToolsManager`: `status()`, `install_missing()`, `check_updates(force)`, `update(tool)`,
   `rollback(tool)` (troca current ↔ previous), `resolve(tool) -> PathBuf`, `RwLock` de uso
   (`acquire_run()` devolve guard de leitura). Respeita `ytdlpChannel` (trocar de canal ⇒
   instalar a última do novo canal).
6. Runtime JS (§16): `resolve_js_runtime(settings, path_var: &OsStr) -> JsRuntime { kind: deno|node, path }`.
   Detecção no PATH (`which`-like portátil) usando o valor **recebido por parâmetro** (o app passa
   `std::env::var_os("PATH")`; os testes passam um PATH falso — nunca alterar a variável de
   ambiente do processo em testes, pois rodam em paralelo). Node exige `node --version` ≥ 20. Para `auto` sem
   nada no sistema ⇒ instalar Deno gerenciado. Função `js_runtime_arg()` ⇒ `"deno:<path>"`.
7. Agendamento: na inicialização do app, `check_updates` em segundo plano conforme §16
   (intervalos guardados em `kv`), se `autoUpdateTools`. Instala atualizações automaticamente
   (com o lock de escrita) e emite `tools://changed`. Também na inicialização: se faltar
   ferramenta obrigatória (yt-dlp, FFmpeg, runtime JS) ⇒ `install_missing()` em segundo plano
   com eventos de progresso (a partir da F12 o onboarding conduz isso na primeira execução).
8. Processos: helper único `spawn_tool(cmd)` usando `process-wrap` (Job Object no Windows,
   grupo de processos no Unix, `CREATE_NO_WINDOW`, kill-on-drop), usado por todas as fases.
9. CLI: flags globais `--data-dir` e `--tools-dir` (arquitetura §4) e
   `reverb-cli tools status|install [--all|<tool>]|update <tool>|rollback <tool>`.
   `--all` = yt-dlp (canal configurado, padrão stable), Deno, FFmpeg **e fpcalc**.
10. `npm run test:prepare` ⇒ `node scripts/test-prepare.mjs` ⇒ `reverb-cli tools install --all
    --tools-dir .test-tools` (idempotente; pula se já instalado e atualizado). Os testes que
    precisam de ferramentas reais usam `REVERB_TEST_TOOLS_DIR` (padrão `.test-tools`).
    Como `cargo test` não compila o CLI antes de rodar o script, o `test-prepare.mjs` faz
    `cargo run -p reverb-cli --release -- tools install …`.
11. Tauri: comandos `tools_status`, `tools_install_missing`, `tools_check_updates`,
    `tools_update`, `tools_rollback`. (A UI dessas funções vem na F06/F12.)
11b. **bgutil / PO token** — seguir o estudo `plano/anexos/estudos/E4-bgutil-po-token.md`
    (e **somente** ele; não abrir o repositório de origem):
    - `ToolSpec` `bgutil` (§16): instala o plugin (zip intacto em `tools/bgutil/<versão>/plugins/`)
      e o código do servidor (`zipball_url` ⇒ pasta `server/`) e roda
      `deno install --allow-scripts=npm:canvas --frozen` com o **Deno gerenciado** (instalar o
      Deno gerenciado se ainda não existir, mesmo que o runtime do yt-dlp seja o Node do sistema).
      Não é instalado por padrão: só quando `potProvider` precisar (ou `tools install bgutil`).
    - `PotServer` (core): sobe o servidor sob demanda em porta livre de `127.0.0.1`, espera
      `/ping` (≤ 30 s), compartilha entre jobs, encerra após 10 min sem uso e ao fechar o app,
      reinicia uma vez se morrer; `pot_args() -> Option<Vec<String>>` devolve
      `--plugin-dirs <pasta> --extractor-args youtubepot-bgutilhttp:base_url=http://127.0.0.1:<porta>`
      quando o provedor está ativo.
    - Política `potProvider` (§6): `auto` liga por 24 h quando a autocura pedir (F04), `always`,
      `off`; estado em `kv`.
12. Gravar fixtures: um `SHA2-256SUMS` real, um `.sha256sum` real do Deno, um `checksums.sha256`
    real do FFmpeg e uma resposta JSON real de `releases/latest` de cada repo em
    `tests/fixtures/github/` (para os testes offline com wiremock).

## Testes de verificação

| # | Teste | Rede |
|---|-------|------|
| T1 | Seleção de asset para (windows,x86_64) e (linux,x86_64) de cada ferramenta usando as respostas gravadas | não |
| T2 | Parsers de checksum com os 3 arquivos reais gravados | não |
| T3 | Parse/comparação de versões: `2026.08.19` < `2026.09.27.232945`; `v2.9.7` vs `deno 2.9.7 (stable…)` iguais; FFmpeg por data | não |
| T4 | Instalação completa **offline**: wiremock serve um "release" falso cujo asset é um zip contendo o `fake-tool` renomeado + checksum correto ⇒ instala, teste de fumaça lê a versão (`FAKE_TOOL_VERSION`), manifesto atualizado | não |
| T5 | Checksum errado ⇒ erro `checksum_mismatch` e manifesto **inalterado** | não |
| T6 | Atualização para versão 2 e depois `rollback` ⇒ current volta à 1; só 2 versões mantidas após a 3ª instalação | não |
| T7 | Lock: com guard de leitura ativo, `update` emite `waiting_jobs` e só conclui após soltar o guard | não |
| T8 | `resolve_js_runtime`: matriz com PATH falso (diretório temporário com `fake-tool` renomeado para `deno`/`node`) cobrindo `auto`, `system-node` com versão 18 (rejeita) e 22 (aceita) | não |
| T9 | Morte de árvore: `spawn_tool(fake-tool --spawn-child-sleep 60)`, matar ⇒ pai **e filho** mortos em ≤ 3 s (verificar PIDs com `sysinfo`) | não |
| T10 | (rede) Instalar ferramentas reais em diretório temporário: `yt-dlp --version` = `tag_name` da API; `deno --version` ok; `ffmpeg -version` e `ffprobe -version` ok | sim |
| T11 | (rede) yt-dlp + Deno gerenciado: `yt-dlp -v --simulate --js-runtimes deno:<path> <FX1>` ⇒ código 0 e stderr contém `JS runtimes: deno-` | sim |
| T12 | (rede) Troca de canal para nightly instala versão com 4 componentes de data; voltar para stable reinstala a estável | sim |
| T12b | bgutil (offline): montagem de `pot_args`; política `auto` liga e expira em 24 h (relógio injetado); ciclo de vida do `PotServer` com um "servidor" falso (`fake-tool` estendido com `--http-ping <porta>` que responde `/ping`) — sobe, reutiliza, encerra por ociosidade, reinicia uma vez se morrer, desiste na 2ª morte | não |
| T12c | (rede) bgutil real em diretório temporário: instalar; subir servidor; `yt-dlp -v --simulate --plugin-dirs … --extractor-args "youtubepot-bgutilhttp:base_url=…" --extractor-args youtube:player_client=mweb <FX2>` ⇒ código 0 e stderr contém `Retrieved a gvs PO Token`; encerrar ⇒ nenhum processo `deno` órfão (`sysinfo`) | sim |
| T13 | CI (Linux): T1–T9 passam no `ubuntu-22.04` **e** o `npm run test:prepare` do CI instala as ferramentas Linux reais, seguido de um passo `reverb-cli tools status` que confirma que `yt-dlp_linux --version`, `deno --version`, `ffmpeg -version` e `fpcalc -version` executam no Linux. (Verificado quando o CI existir, na F06 — registrar como pendência até lá; a F06 só conclui com isso verde.) | — |

## Portão
`npm run test:prepare` ✔ · `npm run verify` ✔ · `npm run e2e` ✔ · `npm run verify:net` (T10–T12, T12c) ✔

## Armadilhas
- A API do GitHub responde 403 sem `User-Agent`.
- `browser_download_url` redireciona para outro host; o `reqwest` segue redirecionamentos por padrão — mantenha.
- No Windows, substituir um `.exe` em uso falha: por isso versões ficam em pastas separadas e
  só o manifesto troca.
- FFmpeg-Builds usa tag rolante `latest`: nunca compare por tag.
- Antivírus pode bloquear temporariamente o executável recém-extraído: o teste de fumaça deve
  tentar de novo até 3× com 1 s de intervalo antes de falhar.

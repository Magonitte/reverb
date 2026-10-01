# PROGRESS — Reverb 3

> Registro vivo mantido pela LLM executora. Atualize ao começar e ao terminar cada sessão,
> e sempre que um portão rodar. Formato definido em `plano/00-protocolo-de-execucao.md` §8.

## Situação atual

- **Fase atual:** F03 — NÃO INICIADA (F00, F01 e F02 concluídas)
- **Último ponto de parada:** F02 concluída e com tag `fase-02-ok`; próximo passo: ler `plano/fases/F03-motor-de-download.md`
- **Pendências humanas abertas:** nenhuma
- **Pendência técnica (não humana):** F02/T13 — o job Linux do CI (`test:prepare` + `scripts/verify-tools.mjs`) só roda depois do remoto GitHub da F06; a F06 só conclui com ele verde (ver seção F02).

## Ambiente (preenchido na F00, 2026-10-01)

| Item            | Versão                                                                        |
| --------------- | ----------------------------------------------------------------------------- |
| node            | v24.14.0                                                                      |
| npm             | 11.12.0                                                                       |
| rustc / cargo   | 1.95.0 / 1.95.0                                                               |
| git / gh        | 2.54.0.windows.1 / 2.101.0                                                    |
| MSVC / WebView2 | Visual Studio Community 2022 (MSVC presente) / WebView2 Runtime 154.0.4258.48 |

## Decisões e desvios do plano

| Data       | Fase | Decisão/desvio                                                                                | Motivo                                                                                                                    |
| ---------- | ---- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| 2026-10-01 | F00  | O código é criado em `D:\Documentos\4 - Pessoal\Reverb` (pasta atual), não em `Reverb_claude` | `Reverb_claude` não existe; o plano já está nesta pasta e o projeto antigo foi movido para `Reverb_old` (somente leitura) |

---

## Modelo de seção por fase (copiar para cada fase)

### FNN — <nome>

- **Status:** NÃO INICIADA | EM ANDAMENTO | BLOQUEADA | CONCLUÍDA
- **Início / fim:**
- **Tarefas:** [ ] 1 · [ ] 2 · …
- **Portão (última rodada):** data · Rust N testes · Vitest N · Playwright N · rede N · app real N · resultado
- **Falhas e correções:**
  - <teste> — causa raiz — correção — ciclos
- **Desvios do plano:**
- **Pendências humanas:**
- **Commit/tag:**

---

## Registro das fases

### F00 — Fundação

- **Status:** CONCLUÍDA
- **Início / fim:** 2026-10-01 / 2026-10-01
- **Tarefas:** [x] 1 ambiente · [x] 2 git · [x] 3 frontend · [x] 4 IPC · [x] 5 app mínimo · [x] 6 workspace · [x] 7 reverb-core · [x] 8 reverb-cli · [x] 9 fake-tool · [x] 10 src-tauri · [x] 11 qualidade · [x] 12 scripts · [x] 13 Vitest · [x] 14 Playwright · [x] 15 CI
- **Portão (última rodada):** 2026-10-01 · Rust 20 testes (core 11, cli 2, fake-tool 7) · Vitest 10 · Playwright 1 · rede 0 (não há) · app real 0 (F07) · `cargo build -p reverb` OK · autoteste headless OK · `npm run verify` OK em ~22–35 s (inclui recriar `dist/` apagado, T11)
- **Falhas e correções:**
  - T7 (lint i18n) — causa raiz: (a) nomes de componente em maiúsculas (`A`) são ignorados pela regra e (b) o modo `jsx-text-only` não inspeciona atributos — correção: componentes de teste com nome `Comp` e `mode: "jsx-only"` — 1 ciclo
  - clippy `zombie_processes` no `fake-tool` — o pai não aguardava o filho — correção: `child.wait()` após o sono — 1 ciclo
- **Desvios do plano:** ver tabela de decisões acima (pasta de trabalho, modo do plugin i18next, versões).
- **Pendências humanas:** nenhuma. Observação: o CI (`ci.yml`) só roda depois do remoto GitHub da F06; o job Linux não foi exercitado ainda.
- **Commit/tag:** `feat(F00): fundação do monorepo` · tag `fase-00-ok`

### F01 — Banco, configurações, logs e tipos TS

- **Status:** CONCLUÍDA
- **Início / fim:** 2026-10-01 / 2026-10-01
- **Tarefas:** [x] 1 db + migração 0001 + triggers FTS5 · [x] 2 settings (macro gera `Settings`/`SettingsView`/`SettingsPatch`) · [x] 3 `default_music_dir`/`resolve_output_dir` · [x] 4 logging (rotação diária, 14 arquivos, redação) · [x] 5 bindings ts-rs (`AppInfo`, `SettingsView`, `SettingsPatch`, `SecretsStatus`, `CoreError`, enums) · [x] 6 Tauri `AppState`/`TauriSink`/comandos · [x] 7 store zustand + mock · [x] 8 CLI `settings get/set`
- **Portão (última rodada):** 2026-10-01 · Rust 68 testes (core 55 + logging_init 1, cli 5, fake-tool 7) · Vitest 16 · Playwright 1 · rede 0 · app real 0 · `npm run verify` OK em 71 s · `npm run e2e` OK · autoteste headless OK
- **Falhas e correções:** só erros de compilação do próprio desenvolvimento (borrow em teste, move de `key` no CLI, re-export de comandos Tauri exige caminho `commands::settings::...`); nenhuma falha de teste do portão.
- **Desvios do plano:**
  - Enums das configurações (`theme`, `language`, `transparency`, `splitChapters`, `cookiesSource`, `ytdlpChannel`, `jsRuntime`, `potProvider`) são enums Rust reais (TS vira união de literais) em vez de `String` validada.
  - `CoreError` ganhou `Db` e `Internal`; o TS `CoreError` vem de `ErrorPayload` (`#[ts(rename = "CoreError")]`), pois o enum usa `Serialize` manual.
  - `globalShortcut` tem só validação sintática (modificador + tecla); a F12 confere com o plugin de atalho global.
  - `Db::call_blocking` (síncrono) adicionado para testes/CLI além do `call` assíncrono.
  - Registrar segredos para redação é aditivo e global ao processo (nunca remove; valores com < 4 caracteres são ignorados).
- **Pendências humanas:** nenhuma.
- **Commit/tag:** `feat(F01): banco, configurações, logs e tipos TS` · tag `fase-01-ok`

### F02 — Gerenciador de ferramentas

- **Status:** CONCLUÍDA
- **Início / fim:** 2026-10-01 / 2026-10-01
- **Tarefas:** [x] 1 cliente GitHub (cache 1 h em memória + `kv`, base URL injetável) · [x] 2 `ToolSpec` (yt-dlp stable/nightly, Deno, FFmpeg+ffprobe, fpcalc, bgutil) · [x] 3 parsers de checksum/versão e comparações · [x] 4 instalação (staging, progresso, hash, extração, `chmod`, teste de fumaça, troca atômica, limpeza) · [x] 5 `ToolsManager` (`status`, `install_missing`, `check_updates`, `update`, `rollback`, `resolve`, `acquire_run`) · [x] 6 runtime JS (`detect_js_runtime`/`resolve_js_runtime`) · [x] 7 agendamento na inicialização (`background_startup`) · [x] 8 `spawn_tool`/`run_capture`/`kill_tree` · [x] 9 CLI `tools status|install|update|rollback` · [x] 10 `test:prepare` · [x] 11 comandos Tauri + mock + wrappers TS · [x] 11b bgutil (`PotPolicy`, `PotServer`, `pot_args`, instalação) · [x] 12 fixtures reais em `tests/fixtures/{github,checksums}/`
- **Portão (última rodada):** 2026-10-01 · Rust 179 testes (core 163, cli 8, fake-tool 8; +4 de rede ignorados) · Vitest 20 · Playwright 1 · rede 4 (T10, T11, T12, T12c) · app real 0 (F07) · `npm run test:prepare` OK (1,2 s com tudo instalado; 1 min 43 s na 1ª vez) · `npm run verify` OK em 139 s · `npm run e2e` OK · `npm run verify:net` OK em 33 s (ferramentas já em cache local) · autoteste headless OK
- **Falhas e correções:**
  - T12c (bgutil real) — causa raiz: o servidor do Deno morria com `NotCapable: Requires read access to …\node_modules\.deno\jsdom…` porque `--allow-read=.` é comparado com o caminho REAL dos arquivos e o diretório de trabalho estava no formato curto do Windows (`C:\Users\JEANCA~1\…`, vindo de `TEMP`) — correção: `normalize_dir` (canonicaliza e tira o prefixo `\?\`) em `bgutil_server_config`; o servidor passou a gravar `server.log` ao lado do código (diagnóstico para a F12) — 1 ciclo
  - Fora isso, só erros de compilação do próprio desenvolvimento; nenhuma falha de teste do portão.
- **Desvios do plano:**
  - `CoreError` ganhou `Coded { kind, message }` (kinds estáveis: `checksum_mismatch`, `checksum_missing`, `tool_missing`, `asset_not_found`, `binary_not_found`, `smoke_test_failed`, `no_previous_version`, `js_runtime_missing`, `pot_unavailable`, `github_http`, `github_rate_limit`, `network`…) e `From<reqwest::Error>` (sem URL na mensagem).
  - `install_missing` cobre yt-dlp, FFmpeg e o runtime JS; fpcalc é opcional e o bgutil só entra com `potProvider = always` ou `tools install bgutil`.
  - `ToolsManager::install` = “instala a última se faltar/estiver defasada” (usa o cache de 1 h); `update` = o mesmo consultando o GitHub sem cache. Trocar o canal do yt-dlp (`manifest.channel != ytdlpChannel`) sempre reinstala, mesmo que a versão do outro canal seja mais antiga.
  - FFmpeg: a pasta da versão é o `updated_at` do asset sem `:`/`-` (ex.: `20260930T190059Z`) e `Installed.reported_version` guarda o que o binário imprime. No manifesto, `current`/`previous` aninham `{version, reported_version, channel, installed_at, asset_updated_at}` (o plano listava as chaves soltas).
  - Deno do sistema só vale com versão ≥ 2 (além do Node ≥ 20 pedido pelo plano), pois o yt-dlp exige Deno 2+.
  - O `.sha256sum` do Deno no Windows vem no formato do `Get-FileHash` (hash em MAIÚSCULAS, várias linhas); o parser pega o 1º token de 64 hex e compara em minúsculas.
  - bgutil: o plugin (zip intacto) e o código do servidor ficam direto em `tools/bgutil/<versão>/` e o `deno install` roda lá (o Deno usa junctions absolutas no Windows; mover a pasta depois quebraria), com limpeza se algo falhar.
  - `ToolsConfig.extra_env` (variáveis extras nos testes de fumaça) deixa os testes escolherem `FAKE_TOOL_VERSION` sem mexer no ambiente do processo; o `fake-tool` ganhou versão por arquivo `<exe>.version` (T8: `deno`/`node` com versões diferentes).
  - O CLI aceita `REVERB_GITHUB_API_URL` (testes do CLI contra um GitHub falso) e, com `--tools-dir` sem `--data-dir`, usa banco em memória (nunca toca nos dados reais do usuário).
  - Adicionados `scripts/verify-tools.mjs` e o passo correspondente no `ci.yml` (T13).
  - Dependências novas no core: `reqwest` (rustls, json, stream), `zip`, `sha2`, `hex`, `semver`, `process-wrap` (+ `windows` só no Windows, para `CREATE_NO_WINDOW`), `futures-util`; dev: `wiremock`, `sysinfo`.
- **Pendências humanas:** nenhuma.
- **Pendência técnica:** T13 — o job Linux do CI (T1–T9 em `ubuntu-22.04`, `test:prepare` + `scripts/verify-tools.mjs`) ainda não foi exercitado (não há remoto GitHub até a F06; a F06 só conclui com ele verde).
- **Commit/tag:** `feat(F02): gerenciador de ferramentas` · tag `fase-02-ok`

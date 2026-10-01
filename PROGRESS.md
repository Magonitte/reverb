# PROGRESS — Reverb 3

> Registro vivo mantido pela LLM executora. Atualize ao começar e ao terminar cada sessão,
> e sempre que um portão rodar. Formato definido em `plano/00-protocolo-de-execucao.md` §8.

## Situação atual

- **Fase atual:** F02 — NÃO INICIADA (F00 e F01 concluídas)
- **Último ponto de parada:** F01 concluída e com tag `fase-01-ok`; próximo passo: ler `plano/fases/F02-gerenciador-de-ferramentas.md`
- **Pendências humanas abertas:** nenhuma

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

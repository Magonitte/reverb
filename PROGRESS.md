# PROGRESS — Reverb 3

> Registro vivo mantido pela LLM executora. Atualize ao começar e ao terminar cada sessão,
> e sempre que um portão rodar. Formato definido em `plano/00-protocolo-de-execucao.md` §8.

## Situação atual

- **Fase atual:** F01 — NÃO INICIADA (F00 concluída)
- **Último ponto de parada:** F00 concluída e com tag `fase-00-ok`; próximo passo: ler `plano/fases/F01-banco-e-configuracoes.md`
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

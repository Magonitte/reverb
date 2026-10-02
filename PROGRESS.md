# PROGRESS — Reverb 3

> Registro vivo mantido pela LLM executora. Atualize ao começar e ao terminar cada sessão,
> e sempre que um portão rodar. Formato definido em `plano/00-protocolo-de-execucao.md` §8.

## Situação atual

- **Fase atual:** F08 — CONCLUÍDA; próxima: F09 (F00–F08 concluídas)
- **Último ponto de parada:** F08 completa (T1–T15 verdes; portão numa só rodada). Próximo passo: ler `plano/fases/F09-pos-processamento.md`.
- **Pendências humanas abertas:** nenhuma
- **Pendência técnica:** nenhuma (F02/T13 resolvida: job Linux do CI verde).

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

### F08 — Identificação de metadados com nota de confiança

- **Status:** CONCLUÍDA
- **Início / fim:** 2026-10-01 / 2026-10-02
- **Tarefas:** [x] 1 `metadata::{normalize,score}` · [x] 2 `parse_title` · [x] 3 `content_type` · [x] 4 provedores Deezer/iTunes/MusicBrainz (`trait MetadataProvider`, `Endpoints`, `RateLimiter`) · [x] 5 cache de 30 dias (`CachedProvider`, relógio injetável) · [x] 6 versão oficial E1 (`official.rs`) · [x] 7 passos `resolve_source`/`identify` no pipeline e na fila · [x] 8 override do usuário · [x] 9 comandos `find_official_version`, `metadata_preview`, `metadata_search` · [x] 10 UI (card da versão oficial, painel de metadados com edição, selo de confiança na Atividade) · [x] 11 fixtures HTTP reais (`tests/fixtures/http/`, `scripts/record-http-fixtures.mjs`) e buscas do YouTube Music (`tests/fixtures/ytdlp/search-*`, `analyze-*`, via `scripts/record-fixtures.mjs`)
- **Portão (última rodada):** 2026-10-02 · Rust 486 testes (+108; 94 em `metadata`, 4 da fila com metadados) · Vitest 197 · Playwright 51 · rede 17 (`verify:net`, com `GITHUB_TOKEN` do `gh auth token`; 6 novos: T10d ×2, T13, T14, T15 ×2) · app real 2 (T10, T11) · `npm run verify` OK em 67 s · `npm run e2e` OK em 56 s · `npm run verify:net` OK · `npm run e2e:app` OK (spec em 21 s)
- **Falhas e correções:**
  - `score` (T2) — três faixas esperadas da **minha** tabela estavam mal calculadas (mesmo artista e duração, título diferente = 0,776; clipe Δ 30 s = 0,875; ambas dentro do que o plano exige: < 0,85 e `dur_score > 0`) — corrigi a tabela, a fórmula do §11.2 não mudou — 2 ciclos
  - compilação — o `reqwest` 0.13 trouxe `RequestBuilder::query` atrás da feature `query` — feature ligada em `reverb-core` — 1 ciclo
  - `preview` de FX3 sem a oficial — empate de pontuação entre o iTunes "Never Gonna Give You Up" e "(2022 Remaster)" (a duração do segundo era 0,01 s mais próxima) e o título aplicado saía com "(2022 Remaster)" — desempate em 3 níveis: nota, título idêntico ao buscado, duração mais próxima (código de produção) — 1 ciclo
  - Playwright T7 (reordenar) — meu selo de confiança aninhou o `<p>` do título e quebrou o seletor `p.truncate:first-of-type` — o selo foi para o grupo de ações à direita e a estrutura título/subtítulo ficou como era — 1 ciclo
  - `e2e:app` T11 — com F08 o job de FX3 troca o clipe pela faixa oficial (T15), então `sourceId === "dQw4w9WgXcQ"` nunca vinha; o teste é sobre cancelar, não sobre a fonte — **justificativa (§11 passo 3 / T15)**: o spec liga `preferOfficialAudio: false` durante o teste e restaura no `finally`; nenhuma asserção foi afrouxada — 1 ciclo
  - `scripts/record-fixtures.mjs` regravou FX1–FX4 (mudaram bytes) ao gerar as buscas novas — `git checkout` das quatro fixtures antigas; as novas ficaram. Ver "Pendência": o script regrava tudo, então futuras gravações devem ser conferidas
- **Bases visuais (Playwright) alteradas de propósito:** `flow-preview-dark` e `flow-preview-light` (linha "Metadados / Pré-visualizar metadados" no Preview de FX2, nova na F08). Conferidas a olho: é a única diferença. As outras bases não mudaram.
- **Desvios do plano:**
  - **ISRC inferido só como plano B.** §11 passo 3(a) manda tentar o ISRC primeiro, inclusive o de "um candidato Deezer com score ≥ auto". Mas o 1º resultado do Deezer para FX3 é a gravação da coletânea (`GBARL0600786`, "Reeling In The Decades"), cujo ISRC leva a `-aIiQj79b6Q` e **não** a `lYBUbBu4W08`, que T13/T15 exigem. Por isso o pipeline faz a busca por texto (E1) primeiro e só usa o ISRC inferido pelo Deezer se o texto não achar nada. Um ISRC **informado** ao `find_official_version` (importação, F15) continua indo primeiro, como em E1.
  - `sim`: "um contém o outro inteiro" usa **palavras inteiras** (`Ice` não está em `Police`); o resto do §11.2 é literal.
  - E1 item 5 (visualizações) e item 6 (explícito × limpo) **não** foram implementados: `VideoInfo` não traz contagem de visualizações e `Candidate` não traz o flag explícito. O desempate usa só "oficial vence até 0,08".
  - `Candidate` ganhou `isrc` (o Deezer devolve na busca e em `/track/{id}`); `MetadataResult.candidates` é `Vec<ScoredCandidate { score, candidate }>`; `MetadataResult` também leva `contentType`, `isrc` e `official` (a sugestão/troca de versão oficial).
  - `confidence` do resultado: oficial = 1,0; aplicado = nota do candidato; revisão = nota do melhor; sem decisão = 0,0 (`bucket = none`). Em `other` o `Job.confidence` fica `None`.
  - Busca no YouTube Music analisa os 3 primeiros em paralelo; se nenhum passar, tenta a seção de vídeos, mas aí só aceita faixa com `is_official_track`.
  - `resolve_source` só propaga o **cancelamento**; falha de análise vira "seguir com a URL original" (o download dirá o que houver de errado). Com `metadata_override` o job não troca de fonte (a UI já escolheu a URL no toggle).
  - Cache só guarda respostas **não vazias** (lista vazia pode ser falha HTTP e não deve ficar 30 dias).
  - `Job` ganhou `confidence` e `metadataResult` (colunas `confidence` e `metadata_result_json` já existiam); `repo::save` agora também grava `source_url`/`source_id` (a troca de fonte persiste).
  - `PipelineJob` ganhou `metadata_override` e `fetch_metadata`; `PipelineEvent` ganhou `Analyzing`, `SourceSwitched`, `Identifying`, `Identified`; `DownloadPipeline::with_metadata` e `ToolsPipeline::with_metadata` ligam o serviço. **O CLI continua sem metadados** (usa o pipeline sem serviço); o app usa um único `MetadataService` por processo (limitadores de taxa compartilhados).
  - O estágio `metadata` entra no progresso geral só quando o evento chega (como `converting`), então o progresso dos jobs sem serviço não mudou.
  - T6 testa o `RateLimiter` com o relógio do tokio pausado (5 chamadas ≥ 4 s); a parte com `wiremock` não usa tempo pausado porque o socket real faria o relógio avançar sozinho.
  - `VideoInfo`, `Chapter` e `AudioFormat` ganharam `Deserialize` (o comando `find_official_version` recebe o `VideoInfo` do Preview). `metadata_preview` recebe `PreviewRequest { url, video?, useOfficial? }`.
  - `FakeBackend` (somente testes) ganhou `set_video`, `set_search`, `search_log` e `analyze_log`; o `DoneInfo` dele usa título/duração do vídeo registrado.
  - O arquivo continua nomeado pelo título do download (`done.title`); a nomeação por `{artist} - {title}`/template é da F09 (organize), que passa a usar o `MetadataResult` já pronto no `PipelineOutput.metadata`.
  - Dependência nova: `strsim` (Jaro-Winkler); `reqwest` com a feature `query`.
- **Pendências humanas:** nenhuma.
- **Pendência técnica:** `scripts/record-fixtures.mjs` regrava FX1–FX4 junto com as buscas; ao rodar de novo, conferir `git diff` das fixtures antigas.
- **Commit/tag:** `feat(F08): identificação de metadados com nota de confiança` · tag `fase-08-ok`

### F07 — Fluxo de download na UI

- **Status:** CONCLUÍDA
- **Início / fim:** 2026-10-01 / 2026-10-01
- **Tarefas:** [x] 1 comandos Tauri (`url_classify`, `analyze`, `search`, `pick_folder`, `open_output_dir`, `clipboard_read_text`, `library_reveal`) · [x] 2 barra de comando (`components/command-bar/`) · [x] 3 Preview (Sheet) · [x] 4 Coleção (`/collection`) · [x] 5 Atividade · [x] 6 Início · [x] 7 mock estendido · [x] 8 E2E no app real (WebdriverIO + `tauri-driver`)
- **Portão (última rodada):** 2026-10-01 · Rust 378 testes · Vitest 166 · Playwright 46 · rede 11 (`verify:net`, com `GITHUB_TOKEN` do `gh auth token`) · app real 2 (T10, T11) · `npm run verify` OK em 71 s · `npm run e2e` OK em 49 s · `npm run e2e:app` OK (spec em 15 s, build à parte)
- **Falhas e correções:**
  - axe `list` (serious) na Atividade — o `DndContext` do dnd-kit injeta elementos de acessibilidade ao lado dos filhos, e o `<ul>` ficava com filhos que não são `<li>` — `DndContext`/`SortableContext` passaram para fora do `<ul>` — 1 ciclo
  - T10 no app real: o selo de qualidade do FX1 real é "AAC 130 kbps" (o mock mostra "Opus 129 kbps", como pede o T3); a asserção do spec do app real passou a exigir só o formato `Fonte: <codec> <n> kbps` — a expectativa do teste estava errada, não o produto — 1 ciclo
  - `e2e:app`: o `tar` do Git Bash é o GNU tar (não lê `.zip` e trata `D:` como host); a extração do msedgedriver passou a usar `Expand-Archive` — 1 ciclo
  - lint `react-hooks/set-state-in-effect` na barra de comando — o reset ao esvaziar o texto virou "ajuste de estado na renderização" + invalidação do ticket num efeito — 1 ciclo
- **Bases visuais (Playwright) alteradas de propósito:** `home-{dark,light}`, `home-busy-{dark,light}`, `home-heal-{dark,light}`, `home-mobile-dark` (ações rápidas e "Recentes" no Início), `activity-{dark,light}` e `activity-busy-{dark,light}` (botões de controle, miniaturas e alças de arrastar). Conferidas a olho contra o desenho (§3.1 e §3.4). Bases novas em `flow-ui.spec.ts-snapshots/`: `flow-{search,preview,collection}-{dark,light}`. As miniaturas do YouTube são trocadas por uma imagem fixa (`stubThumbnails`) para as bases não dependerem da rede; as capturas só rodam no Windows (como `visual.spec.ts`). Obs.: `activity-{dark,light}` ficou dentro da tolerância (0,2 %) mesmo sem os botões novos; foi regerada à mão para refletir a tela atual.
- **Desvios do plano:**
  - `AppState` ganhou `backend` (o mesmo `DownloadBackend` dos downloads) para `analyze`/`search`. Erros do motor viram `CoreError::Coded` com o `kind` do `ErrorKind` (`errors.<kind>` traduz na UI); `search` recebe `source` (`ytmusic`|`youtube`) e `query` e pede 15 resultados. "Buscar detalhes dos 3 primeiros" continua na F08.
  - A Coleção é uma rota (`/collection`) alimentada por `stores/flow.ts`; o Preview é um painel montado no `ShellLayout`. `playlistCtx.index` é a posição **1-based** original da faixa na coleção (vira o número da faixa).
  - Opções por job (`JobOptions`) só viajam quando diferem das configurações globais.
  - A lista de perfis da UI é uma constante TS (`lib/profiles.ts`), espelho de `reverb-core::profiles` (sem comando novo).
  - Ação rápida "Analisar playlist" foca a barra e mostra uma dica; "Editar tags de um arquivo" leva a `/tag-editor` (o conteúdo é da F10). "Colar e baixar" lê a área de transferência por `clipboard_read_text` e envia a barra.
  - Mock: FX4 passou a ter as 10 faixas (as 3 primeiras reais; as demais com ids sintéticos); o cenário `busy` continua com 3. A simulação grava `outputPath` ao concluir, para o "abrir pasta" funcionar no navegador. `mockCalls` registra `library_reveal`/`pick_folder` para os testes.
  - `e2e:app` (`scripts/e2e-app.mjs`): baixa o `msedgedriver` direto de `msedgedriver.microsoft.com` pela versão do WebView2 (em `.test-tools/msedgedriver/<versão>/`) em vez de usar o `msedgedriver-tool`; `tauri-driver 2.1.0` instalado com `cargo install --locked`. A pasta de dados e a de saída são temporárias (`REVERB_DATA_DIR`; `outputDir` via `reverb-cli settings set`) e apagadas no fim. O T11 liga o limite de velocidade (0,1 MB/s) pelo `invoke` da própria página e o desliga no `finally`. Só roda no Windows; o Linux fica com o CI.
  - Dependências novas: `@dnd-kit/{core,sortable,utilities}`; dev: `webdriverio` + `@wdio/{cli,local-runner,mocha-framework,spec-reporter}`; Rust (`src-tauri`): `tauri-plugin-{dialog,opener,clipboard-manager}` e `tokio-util`. Os plugins só são usados em Rust, então as *capabilities* não mudaram.
- **Pendências humanas:** nenhuma.
- **Commit/tag:** `feat(F07): fluxo de download na UI` · tag `fase-07-ok`

### F06 — Atualização automática do app e pipeline de release

- **Status:** CONCLUÍDA
- **Início / fim:** 2026-10-01 /
- **Tarefas:** [x] 1–9 todas
- **Portão:** `npm run verify` OK · `npm run e2e` 36 · T4 CI verde (run 36938667940; antes, 36934994374) · T5/T6 releases v0.1.0 e v0.1.1 com `.exe`, `.AppImage`, `.deb`, `.sig` e `latest.json` (windows-x86_64 e linux-x86_64) · T7 instalado em `%LOCALAPPDATA%\Reverb
everb.exe` (check: available false) · T8 check = available true 0.1.1 (o `latest.json` demorou alguns minutos para propagar) · T9 `--headless-update-install` saiu 0 e o selftest passou a 0.1.1.
- **Decisões:** repo `Magonitte/reverb` público; licença GPL-3.0 (`LICENSE`, Cargo `GPL-3.0-only`); chave em `%USERPROFILE%\.tauri
everb.key` e senha em `reverb.key.password` (guarde cópia fora do repo).
- **Falhas e correções:** testes de árvore de processos no Linux (zumbi contava como vivo; kill-on-drop só matava o líder, agora `KillGroupOnDrop`) · CI sem `cargo build -p reverb` antes do autoteste · delimitador `NOTES` do release.yml · `release.mjs` sem `shell` no Windows.
- **T10:** o usuário abriu o app 0.1.1 e confirmou a aba Atualizações (ferramentas: yt-dlp e FFmpeg atualizados; Deno aparece "Não instalado", a investigar na F07). A versão de teste continua instalada por pedido do usuário.
- **Desvios do plano:** `is_newer` filtra de novo a resposta do plugin (semver estritamente maior).

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

### F03 — Motor de download (yt-dlp + conversão)

- **Status:** CONCLUÍDA
- **Início / fim:** 2026-10-01 / 2026-10-01
- **Tarefas:** [x] 1 `urlkind::classify` · [x] 2 `ytdlp::args` (snapshots `insta`) · [x] 3 `ytdlp::parse` + modelos (`VideoInfo`, `CollectionInfo`, `SearchResult`, todos com TS) · [x] 4 `ytdlp::errors::classify_stderr` · [x] 5 `YtDlpRunner` (`analyze`, `search`, `download`; watchdog, cancelamento, últimas 200 linhas de stderr) · [x] 6 `transcode` (`convert` com `-progress pipe:1`, `probe`) · [x] 7 `profiles` · [x] 8 `workspace` (`JobWorkspace`, `sweep_orphans`) · [x] 9 `organize::sanitize` · [x] 10 `DownloadBackend` + `YtDlpProcessBackend` · [x] 11 CLI `analyze`/`search`/`download` · [x] 12 fixtures `tests/fixtures/ytdlp/` (`scripts/record-fixtures.mjs`)
- **Portão (última rodada):** 2026-10-01 · Rust 293 testes passando (+9 de rede ignorados por desenho) · Vitest 20 · Playwright 1 · rede 9 (T10–T12c da F02 + T13–T17) · app real 0 (F07) · `npm run verify` OK em 142 s · `npm run e2e` OK · `npm run verify:net` OK (com `GITHUB_TOKEN` do `gh auth token`, ver abaixo)
- **Falhas e correções:**
  - T10 (transcode) `mp3_v0` deu 151 kbps (esperado 180–330) — causa raiz: `anoisesrc` é mono e `-ac 2` só duplica o canal; o joint stereo do LAME derruba o VBR. Correção **na fonte de teste, sem mexer nas tolerâncias**: ruído rosa estéreo de verdade (dois `anoisesrc` com `seed` diferentes + `amerge`). Com isso: V0 ≈ 244, 320k ≈ 324, aac_256 ≈ 260, opus_96 ≈ 89 kbps — 1 ciclo
  - T5 (stderr real "unavailable") — causa raiz: o yt-dlp real imprime `This video is unavailable`, que nenhum padrão do §9 cobria — correção: padrão adicionado em `unavailable` — 1 ciclo
  - FX1 `best_audio_abr` — expectativa minha estava errada (o maior bitrate é o AAC 140 ≈ 130 kbps, não o Opus 251) — teste corrigido — 1 ciclo
  - Warnings do ts-rs com `#[serde(alias)]` (quebrariam o clippy `-D warnings`) — correção: structs "raw" só de entrada (`RawChapter`, `RawDone`) — 1 ciclo
  - `verify:net` falhou uma vez com `github_rate_limit` (HTTP 403 anônimo, 60/h esgotado pelas rodadas seguidas dos testes da F02/F03) — externo; reexecutado com `GITHUB_TOKEN` (passa a ser usado pelo gerenciador, §16) e passou — 1 re-execução
- **Desvios do plano:**
  - §9: `geo_blocked` foi posto **antes** de `unavailable` na ordem das regras e o padrão virou `available in your country|geo.?restrict` (a mensagem real "…has not made this video available in your country" não casava, e "This video is not available in your country" seria engolida por `unavailable`); `unavailable` ganhou `This video is unavailable` (stderr real gravado).
  - T10: fonte de áudio é ruído rosa estéreo real (ver acima), não `-ac 2` sobre fonte mono.
  - `fake-tool` ganhou `FAKE_TOOL_OPTS` (um argumento por linha, ligado ⇒ ignora argumentos desconhecidos, para o runner chamar o falso com a linha de comando real de um yt-dlp) e `--stderr-count N`.
  - O backend não cria o `JobWorkspace`: o arquivo baixado precisa sobreviver até ser convertido/movido, então quem cria é o chamador. Novo `DownloadPipeline` (core) faz baixar ⇒ converter ⇒ mover e apaga a pasta do job em qualquer resultado; o CLI `download` usa o pipeline.
  - `sweep_orphans(data_dir, older_than)`: o app deve usar `Duration::ZERO` na inicialização; o CLI usa 24 h (outro processo pode estar baixando).
  - `YtDlpRunner::new(Option<Arc<ToolsManager>>)`: com gerenciador segura `acquire_run`; sem ele (testes) não segura nada.
  - Busca no YouTube Music devolve o que o yt-dlp traz (sem duração); "buscar detalhes dos 3 primeiros" (§8) é da F08.
  - `UrlKind` normaliza vídeo para `https://www.youtube.com/watch?v=ID` (ou `music.youtube.com` se a origem era o Music, para manter os metadados oficiais) e canal para `…/videos`; `watch?list=…` sem `v` vira coleção.
  - Dependências novas no core: `tokio-util` (CancellationToken), `url`, `unicode-normalization`, `async-trait`; dev: `insta`. No CLI: `tokio-util`, `tokio` com `signal` (Ctrl+C cancela e limpa o tmp).
  - TS: modelos novos sem `optional_fields` (campos opcionais saem `T | null`, como o serde realmente emite).
- **Pendências humanas:** nenhuma.
- **Commit/tag:** `feat(F03): motor de download` · tag `fase-03-ok`

### F04 — Fila persistente, retry e autocura

- **Status:** CONCLUÍDA
- **Início / fim:** 2026-10-01 / 2026-10-01
- **Tarefas:** [x] 1 `queue::model` (`Job`, `JobStatus`, `JobStage`, `EnqueueRequest`, `JobOptions`, `PlaylistCtx`, `MoveTarget`, `QueueState`, `DuplicateHit`, todos com TS) · [x] 2 `queue::repo` (CRUD, `next_queued`, `claim` atômico, `move_job`, `restore`) · [x] 3 `QueueService` (parallelism dinâmico, pausa, cancelamento por token, `cancel_all`, `retry`, `remove`, `clear_finished`, `queueLimit`, throttle 250 ms, move-simples via `DownloadPipeline`) · [x] 4 duplicatas (`jobs` + `library`) · [x] 5 retry/backoff 5/30/120 s · [x] 6 `HealCoordinator` (passo 0 PO token, update, nightly, 1×/h) · [x] 7 limite de velocidade (já repassado por `ToolsContext` a cada job, F03) · [x] 8 comandos Tauri + eventos, fila iniciada no `setup()` · [x] 9 CLI `jobs list`, `queue add`, `queue run [--until-idle]` · [x] 10 `FakeBackend` (somente testes)
- **Portão (última rodada):** 2026-10-01 · Rust 329 testes (36 da fila: T1–T14 + extras) · Vitest 24 · Playwright 1 · rede T15 + T16 (+ F02/F03) OK · app real 0 (F07) · `npm run verify` OK em 158 s · `npm run e2e` OK · `npm run verify:net` OK
- **Falhas e correções:** nenhuma falha de teste do portão (só erros de compilação durante o desenvolvimento: feature `test-util` do tokio, CRLF em arquivos do src-tauri ao editar por script).
- **Desvios do plano:**
  - Sem `trait Step` ainda: a fila executa o `DownloadPipeline` (baixar ⇒ converter ⇒ mover) por trás de `trait PipelineRunner` (`ToolsPipeline` resolve o FFmpeg a cada job). A decomposição em passos nasce na F08, quando existe um segundo passo real.
  - `unknown` e `ffmpeg` repetem uma única vez (`min(maxAttempts, 2)` tentativas), como na tabela §9; `network` usa `maxAttempts`.
  - `attempts` conta tentativas iniciadas; reenfileirar pela autocura devolve a tentativa. Restauração mantém `attempts` (a tentativa interrompida fica contada).
  - Autocura: um `epoch` sobe a cada conserto; job que falhou com um `epoch` mais antigo só é reenfileirado (o conserto já aconteceu enquanto rodava), o que garante 1 única chamada a `update` com vários jobs falhando. Escalada stable→nightly acontece dentro da mesma hora (nível 1 em `kv.heal_level`); depois do nightly, nova autocura só após 1 h. `HealTools` é o trait mockado nos testes; `ToolsHeal` é a implementação real.
  - Falha definitiva da autocura grava `error_kind = extractor` e `error_message = "errors.extractorPersistent"` (chave i18n; a UI traduz). Chaves novas em pt-BR/en: `errors.extractorPersistent`, `notices.potEnabled`, `notices.ytdlpNightly`.
  - Duplicata: `CoreError::Coded("duplicate")` com a lista no texto; a UI usa `check_duplicates` para a lista estruturada.
  - Mudança de `parallelism` em tempo real: `settings_update` chama `queue.wake()`; sem polling.
  - `QueueDeps.start_paused` (o CLI `queue add` enfileira sem processar).
  - Frontend: wrappers em `api.ts` + backend mock da fila (`mock/queue.ts`, com testes) para os 12 comandos novos do §15.
  - Pendência de §9 item 6 (volta automática do nightly para o stable na verificação diária) **não** está nas tarefas/testes da F04 e ficou para a F12 (verificação periódica em segundo plano).
  - Dependências novas no core: `uuid` (v4); dev: `tokio/test-util`.
- **Pendências humanas:** nenhuma.
- **Commit/tag:** `feat(F04): fila persistente, retry e autocura` · tag `fase-04-ok`

### F05 — Shell da UI, design system, temas, i18n e backend mock

- **Status:** CONCLUÍDA
- **Início / fim:** 2026-10-01 / 2026-10-01
- **Tarefas:** [x] 1 estudo da referência · [x] 2 estilos · [x] 3 layout · [x] 4 rotas · [x] 5 componentes base · [x] 6 temas e transparência · [x] 7 i18n · [x] 8 atalhos · [x] 9 mock + cenários · [x] 9b `check:ipc` · [x] 10 stores · [x] 11 janela Mica/Linux
- **Estudo da referência (T1):** além dos tokens de `02-design.md` §1, o HTML traz `--dur-slow: 400ms`, corpo em 13px/1.5, fundo da janela `#0a0a10` com dois radiais ambientes, sulcos concêntricos, ícone da titlebar em disco (gradiente radial 18px), barra ativa da sidebar e preenchimento da barra de progresso por estágio (download/convert/done/error). Foram para `tokens.css`/`base.css`: `--dur-slow`, `--window-bg`, `--accent-on`, `--surface-*`, `--track`, `--overlay`, `--scrollbar*`, `--stage-*`, `--bottomnav-h`, sulcos via `repeating-radial-gradient` e Inter Variable no fallback (Linux).
- **Checagem visual (T10):** bases em `tests/e2e/visual.spec.ts-snapshots/` (rotas nos dois temas, busy, heal, overlay Ctrl+K, 390×844). Conferidas contra o HTML: trilho de 58px, âmbar, sulcos, titlebar e barra de comando batem. O protótipo ainda mostra ações rápidas, recentes e itens completos da fila — isso é conteúdo da F07 (§3.1–3.4), não da casca. Telas sem dados ficam em `EmptyState`. Diferença corrigida antes das bases: acento do tema claro escurecido para passar no axe (ver desvios).
- **Portão (última rodada):** 2026-10-01 · Rust 329 (igual à F04; a F05 não adiciona testes Rust) · Vitest 114 · Playwright 35 · rede 0 (não há nesta fase) · app real 0 (F07) · `npm run verify` OK em 294 s · `npm run e2e` OK em 40 s · `cargo build -p reverb` OK · autoteste headless OK
- **Falhas e correções:** nenhuma falha de teste do portão nesta rodada.
- **Desvios do plano:**
  - Tema claro: `--accent` `#b05e12` (plano: `#c46a16`), `--accent-hover`/`--accent-pressed`/`--gold`/`--warning`/`--success` e `--fg-dim` um passo mais escuros, para contraste AA (T9, axe nos dois temas).
  - `analyze`/`search` não são comandos Tauri até a F07. As fixtures FX1–FX4 viraram dados TS (`src/lib/ipc/mock/fixtures.ts`) e alimentam os cenários `busy`/`errors`/`heal`; a simulação temporizada de estágios fica em `queue.ts`.
  - Contador de revisão na sidebar espera a fila de revisão (F10). Nesta fase os indicadores são o badge de jobs ativos e o ponto de atualização de ferramenta em Configurações.
  - Início, nesta fase: título, barra de comando (só foco; a lógica é da F07) e até 3 jobs. Ações rápidas e recentes entram na F07. As outras abas de Configurações são estado vazio; só Aparência grava tema, idioma e transparência.
  - `check:ipc` compara quatro conjuntos (Rust, `COMMANDS`, wrappers e mock) e entrou no `verify`.
- **Pendências humanas:** nenhuma.
- **Commit/tag:** `feat(F05): shell da UI, temas e i18n` · tag `fase-05-ok`

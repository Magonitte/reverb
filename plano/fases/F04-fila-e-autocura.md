# F04 — Fila persistente, retry e autocura

**Objetivo:** fila de downloads robusta: paralela e ajustável, persistente (sobrevive a
fechamento/crash), com pausa, cancelamento, reordenação, retry com backoff, detecção de
duplicatas e **autocura** do yt-dlp quando o YouTube muda.

**Pré-requisito:** F03. **Arquitetura:** §9, §10, §5 (tabela `jobs`), §15.

## Tarefas

1. `reverb-core::queue::model`: `Job` (espelha a tabela, com `TS`), `JobStatus`, `JobStage`,
   `EnqueueRequest { url, source_id?, title?, thumbnail?, duration_s?, profile_id?, options?:
   JobOptions, metadata_override?, playlist_ctx?: PlaylistCtx { playlist_title, playlist_id,
   index, sync_id? }, priority: bool, allow_duplicate: bool }`, `JobOptions` (sobrescritas por
   job: fetch_metadata, fetch_lyrics, fetch_artwork, sponsorblock, auto_organize, output_dir,
   split_chapters).
2. `reverb-core::queue::repo`: CRUD de jobs no banco; `next_queued()` por `position`;
   `move_job(id, before_id|after_id)`; restauração (`running ⇒ queued`).
3. `reverb-core::queue::scheduler` (`QueueService`): loop tokio que inicia jobs até o limite
   dinâmico de `parallelism`; pausa/retomada; cancelamento (token por job); `cancel_all`;
   `retry`; `remove`; `clear_finished`; limite `queueLimit` (erro `queue_full` ao enfileirar além);
   throttle de eventos (§10); pipeline de passos (`trait Step`) com F04 = [download, convert,
   move-simples]; o passo "move-simples" usa `sanitize` + `unique_path` para
   `<outputDir>/<título>.<ext>` (será substituído na F09).
4. Duplicatas: `check_duplicates(source_ids, profile_id)` consulta `jobs` (não falhos/cancelados)
   e `library` (F09 preenche) por `(provider, source_id)`; `enqueue` com duplicata e
   `allow_duplicate = false` ⇒ erro `duplicate` com a lista.
5. Retry/backoff (§9) com relógio injetável (para testes com `tokio::time::pause`).
6. `HealCoordinator` (§9, **incluindo o passo 0 de PO token** — estudo E4): recebe falhas
   `extractor` e `bot_check`, garante execução única, liga o provedor bgutil quando aplicável, chama
   `ToolsManager` (trait para mock nos testes), reenfileira, troca de canal, registra em `kv`,
   emite `heal://state` e `notice`.
7. Limite de velocidade: repassar `speedLimitMbps` ao contexto do yt-dlp.
8. Tauri: comandos da fila (§15) + eventos; `QueueService` iniciado no `setup()` e restaurado do banco.
9. CLI: `reverb-cli jobs list [--json]`, `reverb-cli queue add <url> [--profile]` e
   `reverb-cli queue run --until-idle` (processa a fila e sai quando vazia — usado em testes).
10. `FakeBackend` (somente testes) implementando `DownloadBackend` com roteiro por URL:
    `ok(duração_ms, passos_de_progresso)`, `fail(kind)`, `fail_then_ok(kind, n)`, `hang`.

## Testes de verificação (todos com `FakeBackend`, banco temporário e tempo controlado, salvo indicação)

| # | Teste |
|---|-------|
| T1 | 6 jobs com `parallelism = 2` ⇒ nunca mais de 2 em `running` simultâneos (registrar o máximo observado); todos `done` |
| T2 | Mudar `parallelism` de 1 para 3 durante a execução ⇒ passa a rodar 3 sem reiniciar o app; reduzir para 1 não interrompe os que já rodam |
| T3 | Pausar ⇒ nenhum novo inicia, os em execução terminam; retomar ⇒ continua |
| T4 | Cancelar job em fila e job em execução ⇒ `cancelled`, workspace apagado, contador liberado |
| T5 | `fail_then_ok(network, 2)` com `maxAttempts = 3` ⇒ `done` após esperas de 5 s e 30 s (verificar com relógio pausado); `fail(network)` ⇒ `failed` com `attempts = 3` |
| T6 | Erros permanentes (`unavailable`, `age_restricted`, `bot_check`, `geo_blocked`, `disk`) ⇒ `failed` sem retry |
| T7 | Autocura: 3 jobs falham com `extractor` ao mesmo tempo ⇒ `ToolsManager::update` chamado **1 vez**; jobs reenfileirados sem consumir tentativa; com o backend passando a funcionar ⇒ `done` |
| T8 | Autocura escalando: update não resolve ⇒ troca para nightly (setting persistido, `notice` emitido) ⇒ resolve; se nightly também falhar ⇒ `failed` com `errors.extractorPersistent`; segunda falha dentro de 1 h não dispara nova autocura |
| T8b | Passo 0 (PO token): `fail(bot_check)` com `potProvider = auto` ⇒ provedor ligado (mock do `PotServer`), job reenfileirado sem consumir tentativa e concluído; com o provedor já ligado ⇒ `failed` (`bot_check`) sem novo passo 0; `potProvider = off` ⇒ `failed` direto; `extractor` com stderr `HTTP Error 403` ⇒ passo 0 antes do update do yt-dlp |
| T9 | Restauração: jobs `running` no banco ⇒ ao recriar o serviço viram `queued` e são processados |
| T10 | Reordenar (`move_job`) e "Baixar agora" (`priority`) alteram a ordem de início |
| T11 | `queueLimit` respeitado (erro `queue_full`) |
| T12 | Duplicatas: mesmo `source_id`+perfil ⇒ erro `duplicate`; com `allow_duplicate` ⇒ aceita; perfil diferente ⇒ aceita |
| T13 | Throttle: backend emitindo 100 progressos/s ⇒ ≤ 5 eventos `job://updated`/s por job, mas mudanças de status sempre emitidas |
| T14 | Progresso geral ponderado (§10) monotônico de 0 a 1 |
| T15 | (rede) `reverb-cli queue add` FX1 com perfis `original`, `mp3_v0`, `opus_96` + `queue run --until-idle` com `parallelism 2` ⇒ 3 arquivos corretos, `tmp/` vazio |
| T16 | (rede) Com `speedLimitMbps = 0.2` (MB/s, para o download durar > 10 s), cancelar um download real em andamento (FX2) após o 1º evento de progresso ⇒ `cancelled`, nenhum processo `yt-dlp`/`ffmpeg` filho restante (verificar com `sysinfo`), `tmp/` limpo |

## Portão
`npm run verify` ✔ · `npm run e2e` ✔ · `npm run verify:net` (T15, T16 + anteriores) ✔

## Armadilhas
- Não segure lock do banco nem do estado da fila durante `await` do download.
- Use `tokio::time::pause()` + `advance()` para testar backoff sem esperar de verdade.
- Os eventos devem sair do mesmo lugar (um único `emit_job`) para o throttle ser confiável.

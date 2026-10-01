# F01 — Banco de dados, configurações, logs e tipos TS

**Objetivo:** SQLite com migrações, serviço de configurações (fonte da verdade no backend),
logs redigidos e geração de tipos TypeScript a partir do Rust.

**Pré-requisito:** F00 concluída. **Arquitetura:** §4, §5, §6, §15, §18.

## Tarefas

1. `reverb-core::db`: `Db::open(path)` (cria pasta, PRAGMAs de §5), runner de migrações
   (`migrations/0001_init.sql` exatamente como §5, incluindo os **triggers** do FTS5 no padrão
   external-content: `AFTER INSERT` insere em `library_fts`; `AFTER DELETE` insere
   `('delete', old.rowid, old.title, old.artist, old.album)`; `AFTER UPDATE` faz o delete + insert).
   Execução: `Arc<Mutex<Connection>>` + `tokio::task::spawn_blocking` num helper
   `db.call(|conn| …).await`.
2. `reverb-core::settings`: struct `Settings` com **todas** as chaves de §6 (mesmo as usadas em
   fases futuras — já com padrão), `SettingsPatch` (todos `Option`), `SettingsService` com
   `get()`, `update(patch) -> Result<Settings>` (valida conforme §6, persiste chave a chave em
   transação, emite `settings://changed` pelo `EventSink`), `reset()` (mantém segredos).
   Dois tipos: `Settings` (interno, com todos os campos, inclusive segredos `acoustidKey`,
   `spotifyClientId`, `spotifyClientSecret`, `discogsToken`, `jamendoClientId`) e
   `SettingsView` (o que vai para a UI e para o evento): todos os campos **menos** os segredos,
   mais `secretsStatus: { acoustid: bool, spotify: bool, discogs: bool, jamendo: bool }`.
   A UI nunca recebe os valores. O `SettingsPatch` aceita definir (`Some("valor")`) e limpar
   (`Some("")`) segredos. Só `SettingsView`/`SettingsPatch`/`SecretsStatus` derivam `TS`.
3. `reverb-core::paths`: `default_music_dir()` (§4) e `resolve_output_dir(settings)`.
4. Logs (`reverb-core::logging`): `init(log_dir)` com `tracing-appender` diário, retenção de 14
   arquivos (apagar mais antigos na inicialização), camada de **redação** (§18) aplicada à
   mensagem formatada. Função pura `redact(&str) -> String` testável.
5. `ts-rs`: derivar `TS` em `AppInfo`, `SettingsView`, `SettingsPatch`, `SecretsStatus`, `CoreError`
   (formato serializado). Teste `export_bindings` (ts-rs gera automaticamente com `#[ts(export)]`
   ao rodar `cargo test`). Conferir que `src/bindings/*.ts` foram gerados e commitados.
6. Tauri: estado gerenciado `AppState { db, settings, sink }` criado no `setup()` (usando o
   diretório de dados do core). `TauriSink` implementa `EventSink` com `app.emit`. Comandos
   `settings_get`, `settings_update`, `settings_reset`.
7. Frontend: `stores/settings.ts` (zustand) carrega via `settingsGet()` na inicialização, aplica
   `settings://changed`, expõe `update(patch)` otimista com rollback em erro. Backend mock com
   as mesmas validações básicas (parallelism 1–4 etc.) para testes de UI.
8. CLI: `reverb-cli settings get` / `reverb-cli settings set <chave> <json>` (usado em testes
   do app real nas fases seguintes).

## Testes de verificação

| # | Teste |
|---|-------|
| T1 | Migração em banco vazio ⇒ `user_version = 1`, todas as tabelas existem (consultar `sqlite_master`) |
| T2 | Rodar migrações duas vezes é idempotente |
| T3 | FTS5: inserir 3 linhas em `library` (uma com "Ação Música"), buscar `library_fts MATCH 'acao'` encontra (remove_diacritics), atualizar título reflete na busca, apagar remove da busca |
| T4 | `foreign_keys` ativo: apagar `syncs` cascateia `sync_items` |
| T5 | `Settings::default()` bate com a tabela §6 (teste explícito campo a campo) |
| T6 | Validação: `parallelism` 0 e 5 ⇒ erro; 1 e 4 ⇒ ok; `confidenceAutoApply` ≤ `confidenceReview` ⇒ erro; `outputDir` relativo ⇒ erro; chave desconhecida ⇒ erro |
| T7 | Persistência: atualizar, fechar e reabrir o banco ⇒ valores mantidos |
| T8 | Segredos: `SettingsView` serializado (e o evento `settings://changed`) não contém o valor do segredo; `secretsStatus` correto; limpar segredo funciona; `reset()` mantém segredos |
| T9 | Concorrência: 50 `update` paralelos (tokio) sem erro de "database is locked" |
| T10 | Evento `settings://changed` emitido no `MemorySink` com o novo estado |
| T11 | `redact`: `?key=ABC123&x=1` ⇒ `?key=***&x=1`; `client_secret=xyz` ⇒ `client_secret=***`; valor de chave de API conhecido ⇒ `***`; texto comum intacto |
| T12 | Retenção de logs: criar 20 arquivos antigos falsos ⇒ após `init`, sobram 14 |
| T13 | Bindings atualizados (`npm run check:bindings`) |
| T14 | Vitest: store hidrata do backend mock; `update` otimista + rollback quando o mock rejeita |
| T15 | CLI: `settings set parallelism 3` seguido de `settings get` retorna 3 (teste com `--data-dir` temporário) |

## Portão
`npm run verify` ✔ · `npm run e2e` ✔ · `node scripts/verify-app-headless.mjs` ✔

## Armadilhas
- FTS5 external content exige os triggers exatos; erro comum é esquecer o comando `'delete'`.
- `rusqlite::Connection` não é `Sync`: nunca segure o `Mutex` através de `.await`.
- ts-rs e `serde(rename_all = "camelCase")`: confira que o TS gerado usa camelCase.

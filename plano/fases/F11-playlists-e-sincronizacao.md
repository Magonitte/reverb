# F11 — Playlists sincronizadas

**Objetivo:** o usuário aponta uma playlist (ou álbum, ou canal) e o Reverb mantém uma cópia
local: baixa só o que é novo, respeita pausas para não ser bloqueado, opcionalmente remove o
que saiu da playlist e gera `.m3u8`.

**Pré-requisito:** F10. **Design:** §3.3 (botão sincronizar), §3.8. **Arquitetura:** §5
(`syncs`, `sync_items`), §10, §13 (`{playlist}`, `{playlist_index}`).

## Tarefas

1. `sync` (core): `create(SyncCreate { url, title?, profile_id, output_dir?, interval_hours,
   max_items?, remove_deleted, write_m3u })` (analisa a coleção para obter título/id/miniatura),
   `update`, `delete(id, delete_files)`, `list`, `items(id)`.
2. **Diff** (`plan_sync(current_items, remote_entries, max_items) -> SyncPlan { add, removed,
   reordered, unchanged }`) — função pura. `max_items` considera as N primeiras da playlist
   remota (ordem da playlist).
3. **Execução** `run(id)`: analisa (flat) ⇒ plano ⇒ para cada item de `add` que **já exista na
   biblioteca** com o mesmo `(provider, source_id)` e perfil, apenas vincula (`sync_items.library_id`)
   sem baixar de novo ⇒ enfileira o restante de `add` como `kind=playlist_item`
   com `playlist_ctx { playlist_title, playlist_id, index, sync_id }` ⇒ marca `removed`
   (e, se `remove_deleted`, envia arquivo para a lixeira e remove da biblioteca) ⇒ atualiza
   posições ⇒ grava `last_sync_at`, `last_result_json { added, removed, failed }` ⇒ evento
   `sync://updated`. Itens cujo job falhou podem ser re-tentados na próxima execução.
4. **Pausa entre faixas** de playlist: o scheduler respeita `playlistPacingSeconds` entre
   **inícios** de jobs `playlist_item` (não afeta jobs `single`).
5. **Agendador**: verifica a cada 15 min (e 2 min após abrir o app) quais syncs ativas estão
   vencidas (`interval_hours > 0` e `now - last_sync_at ≥ intervalo`) e roda uma por vez.
6. **.m3u8** (se `write_m3u`): ao terminar todos os jobs da execução, (re)gerar
   `<outputDir>/Playlists/<título sanitizado>.m3u8` em UTF-8: `#EXTM3U`, para cada item
   presente e baixado na ordem da playlist `#EXTINF:<segundos>,<artista> - <título>` +
   caminho **relativo** à pasta do `.m3u8` com `/`. Itens ausentes são omitidos.
7. Comandos: `syncs_list`, `sync_create`, `sync_update`, `sync_delete`, `sync_run`, `sync_items`.
8. **UI**: botão "Sincronizar esta playlist" na Coleção ⇒ diálogo (intervalo: manual, 6 h, 12 h,
   24 h, semanal; pasta; perfil; limitar às N primeiras; remover excluídas; gerar .m3u8);
   tela Playlists e detalhe (design §3.8); faixa no Início "N playlists sincronizadas · próxima
   em X"; notificação-resumo ("Playlist X: 3 novas faixas") — notificação nativa vem na F12, aqui toast.
9. Mock estendido (sync de FX4 com estados variados).
10. **Casos de borda do estudo `plano/anexos/estudos/E2-sincronizacao-bordas.md`**: renomear (não
    baixar de novo) quando a posição/título da playlist mudar e o modelo usar `{playlist_index}`
    ou `{playlist}` (com o `.lrc` junto); faixa duplicada na playlist ⇒ usar a 1ª ocorrência e
    avisar; itens que falharam são re-tentados na próxima execução; `.m3u8` regenerado sempre.
    (O atalho por `checksum` do Deezer entra na F15, junto com as fontes Deezer.)

## Testes de verificação

| # | Teste | Ferramenta |
|---|-------|-----------|
| T1 | `plan_sync`: tabela ≥ 10 casos — primeira sincronização; nada mudou; 1 adicionada no meio; 1 removida; reordenação; id duplicado na playlist; `max_items` menor que o total; item que voltou após removido | cargo test |
| T2 | Execução com `FakeBackend` e fixture FX4 (flat), com 1 das faixas já na biblioteca (mesmo perfil) ⇒ 9 jobs e 1 vínculo sem download; segunda execução ⇒ 0 jobs; fixture modificada (−1, +1) ⇒ 1 job e 1 `removed`; com `remove_deleted` ⇒ arquivo na lixeira e fora da biblioteca | cargo test |
| T3 | Pausa entre faixas: com `playlistPacingSeconds = 3` e paralelismo 2, inícios de jobs da playlist espaçados ≥ 3 s (tempo pausado); jobs `single` não são atrasados | cargo test |
| T4 | Agendador: com relógio injetado, sync de 24 h roda quando vencida e não antes; `interval_hours = 0` nunca roda sozinha; `enabled = false` não roda | cargo test |
| T5 | `.m3u8`: snapshot do conteúdo (ordem, `#EXTINF`, caminhos relativos com `/`, UTF-8 com acentos); item ausente omitido | cargo test |
| T5b | E2: mudança de posição com `{playlist_index:03}` no modelo ⇒ arquivo e `.lrc` renomeados, **zero** novos downloads (contador do `FakeBackend`); duplicata na playlist ⇒ 1 download + aviso; item falho na execução 1 é re-enfileirado na execução 2 | cargo test |
| T6 | UI: diálogo de sincronização valida campos e chama `sync_create`; tela Playlists mostra status/última/próxima; "Sincronizar agora"; excluir pergunta sobre arquivos | Vitest + Playwright |
| T7 | (rede) Criar sync de FX4 com `max_items = 2` e `write_m3u` ⇒ 2 arquivos baixados e organizados; `.m3u8` com 2 entradas que apontam para arquivos existentes; segunda execução não baixa nada | `verify:net` |
| T8 | (rede) Coleção pela URL FX5 (`MPREb_…`) cria sync equivalente a FX4 (mesmo `playlist_id`) | `verify:net` |
| T9 | **App real**: criar sync pela UI (com `max_items = 1`) ⇒ "Sincronizar agora" ⇒ item baixado aparece no detalhe da playlist | WebdriverIO |

## Portão
`npm run verify` ✔ · `npm run e2e` ✔ · `npm run verify:net` ✔ · `npm run e2e:app` ✔

## Armadilhas
- Playlists grandes: a análise flat pode levar minutos; mostrar progresso indeterminado e não
  bloquear a UI.
- O YouTube pode ter vídeos indisponíveis na playlist (entradas com título "[Private video]" ou
  "[Deleted video]"): ignorá-los no plano e mostrar contagem de "indisponíveis".

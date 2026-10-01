# F15 — Importação de playlists, seguir artistas, "Faltando" e qualidade-alvo

**Objetivo:** trazer música de fora do YouTube para a coleção — colar uma playlist/álbum do
Deezer ou do Spotify e o Reverb encontra cada faixa oficial no YouTube Music (por ISRC) —,
seguir artistas para receber lançamentos novos, ver o que falta da discografia e manter a
qualidade num alvo automaticamente.

**Pré-requisito:** F14. **Estudos (leitura obrigatória):** `E1-matching-isrc.md`,
`E2-sincronizacao-bordas.md` (checksum), `E3-importacao-playlists.md`,
`E5-colecao-seguir-artistas.md`. **Arquitetura:** §5 (`syncs.provider`, `sync_items` novos
campos, `followed_artists`, `followed_releases`, `library.isrc`), §6 (`qualityTargetKbps`,
`autoUpgrade`, `artistCheckIntervalHours`, `spotifyClientId/Secret`), §14, §15.
**Lembrete de sala limpa:** implementar só a partir dos estudos (protocolo §7, regra 8b).

## Tarefas

1. **Fontes de importação** (`reverb-core::import`): `trait ImportSource { matches(url);
   async fetch(url) -> ImportedCollection { provider, id, title, cover, tracks: Vec<ImportedTrack>,
   checksum? } }`. Implementações `DeezerSource` (sem chave; playlist, álbum, faixa, link curto com
   redirecionamento) e `SpotifySource` (Client Credentials com as credenciais **do usuário**;
   playlist, álbum, faixa; tratamento de 401/403/404/429 conforme E3). Paginação completa.
2. **Casamento** (`match_imported(track) -> MatchResult { video_id?, confidence, via: isrc|text,
   bucket: ok|review|none }`) usando E1 (ISRC primeiro). Execução em paralelo limitado (3) com o
   limitador de taxa. Resultado com metadados **da fonte** (prioridade sobre a identificação —
   E3, item 3) e o ISRC.
3. **Comandos** `import_analyze(url)` (devolve a coleção + casamento por faixa, com progresso por
   evento `import://progress`) e `import_enqueue(selection, mode: once|sync, sync_options?)`.
   Faixas `none` não são enfileiradas; `review` são enfileiradas só se o usuário marcar.
4. **Sincronização de fontes externas** (estende a F11): `syncs.provider = deezer|spotify`;
   `sync_items.source_id` = ID da faixa na fonte; `matched_video_id`/`match_confidence`/`meta_json`
   preenchidos; o diff usa o ID da fonte; casamento só para faixas novas; atalho por `checksum`
   do Deezer (E2 item 7) guardado em `last_result_json`.
5. **Seguir artistas** (E5): `artists_search(nome)` (Deezer), `artist_follow(provider_artist_id,
   options)`, `artist_update`, `artist_unfollow(id, delete_files)`, `artists_followed`,
   `artist_releases(id)` (lista com estado: completo / incompleto x de y / ausente /
   não monitorado), `artists_check_now`. Agendador a cada `artistCheckIntervalHours` (e 5 min
   após abrir o app, se vencido): busca discografia, grava `followed_releases`, aplica
   `monitor_existing` (na primeira vez) e `monitor_new` (lançamentos com `release_date` posterior
   ao `last_check_at`), filtros de tipo/variantes, deduplicação single×álbum por ISRC, e
   enfileira faixas com `playlist_ctx` = álbum (para numeração e organização corretas).
   `monitor_new = notify` ⇒ notificação + item em "Faltando".
6. **"Faltando"**: `missing_list(artist_id?)` cruza faixas esperadas (por ISRC, senão por título
   normalizado + duração ±3 s) com a biblioteca; `missing_download(release_ids)`.
7. **Qualidade-alvo** (E5 §3): com `qualityTargetKbps > 0` e `autoUpgrade`, o agendador diário
   roda `upgrade_scan` (F13) só para itens abaixo do alvo e enfileira melhorias (respeitando a
   pausa entre faixas). Nunca rebaixa.
8. **UI**:
   - Barra de comando reconhece URLs Deezer/Spotify ⇒ tela **Coleção** em modo importação, com
     coluna de casamento por faixa (✓ confiança, ⚠ revisar, ✗ não encontrada), filtros por estado,
     botões "Baixar selecionadas" e "Sincronizar esta playlist".
   - URL de artista Deezer ou ação "Seguir artista" (Biblioteca, resultado de busca) ⇒ diálogo
     com as opções de E5 (o que baixar agora, lançamentos novos, tipos, excluir variantes,
     perfil, pasta).
   - Biblioteca ganha as abas **Artistas seguidos** (cards com foto, nº de lançamentos,
     completos/incompletos, próxima verificação) e **Faltando**.
   - Configurações: Metadados › Spotify (credenciais + guia passo a passo para criar o app);
     Downloads › Qualidade-alvo (alvo, melhorar automaticamente); Integração › intervalo de
     verificação de artistas.
   - i18n completo; mock estendido (coleções Deezer/Spotify de exemplo, artista com 10
     lançamentos).
9. Gravar respostas reais em `tests/fixtures/http/deezer/` (playlist, álbum, faixas de álbum,
   artista, discografia paginada, erro 800) e montar fixtures Spotify a partir da documentação
   oficial (token, playlist, 404, 429 com `Retry-After`).

## Testes de verificação

| # | Teste | Rede |
|---|-------|------|
| T1 | Classificação de URLs Deezer/Spotify (≥ 15 casos, inclui `intl-pt`, `?si=`, link curto via wiremock com redirecionamento, URL inválida) | não |
| T2 | `DeezerSource` com fixtures: playlist paginada (montar 2 páginas), álbum (faixas com `isrc`/posição/disco), erro 800 ⇒ mensagem i18n | não |
| T3 | `SpotifySource` com wiremock: obtenção e renovação de token, paginação, 404 em playlist editorial ⇒ mensagem específica, 429 respeita `Retry-After` (relógio injetado), sem credenciais ⇒ erro `spotify_credentials_missing` | não |
| T4 | Casamento: fixtures de busca por ISRC (1 resultado oficial ⇒ aceita direto), vários resultados ⇒ score, sem resultado ⇒ texto; metadados da fonte prevalecem sobre a identificação | não |
| T5 | Sync de fonte externa com `FakeBackend`: 1ª execução enfileira casadas; 2ª com `checksum` igual ⇒ nenhuma análise de itens; fonte com −1/+1 faixa ⇒ 1 casamento novo, 1 removida | não |
| T6 | Seguir artista: filtros de tipo/variantes (≥ 10 casos), dedupe single×álbum por ISRC, `monitor_existing` (all/latest/none) na 1ª verificação, `monitor_new` (all/notify/none) com relógio injetado, `playlist_ctx` de álbum gera numeração correta no modelo de nomes | não |
| T7 | "Faltando": álbum completo, incompleto (x de y), ausente; casamento por ISRC e por título+duração | não |
| T8 | Qualidade-alvo: abaixo do alvo + autoUpgrade ⇒ enfileira; no alvo ⇒ nada; autoUpgrade desligado ⇒ nada; nunca rebaixa | não |
| T9 | UI: modo importação da Coleção (estados de casamento, filtros, seleção), diálogo de seguir artista, abas Artistas/Faltando, guia de credenciais do Spotify; axe; screenshots inspecionadas | Vitest + Playwright |
| T10 | (rede) Deezer real: playlist obtida de `GET https://api.deezer.com/chart/0/playlists?limit=1` ⇒ todas as faixas com `isrc`; casar as 3 primeiras ⇒ ≥ 2 com confiança ≥ 0.85 | sim |
| T11 | (rede) Seguir "Rick Astley" (Deezer `6160`) com `latest` e tipo álbum, limite de teste de 3 faixas ⇒ 3 jobs com `via = isrc` e confiança ≥ 0.85 em ≥ 2; baixar 1 e conferir `library.isrc` gravado | sim |
| T12 | (rede, só se o usuário fornecer credenciais do Spotify) importar uma playlist pública do usuário ⇒ faixas com `isrc`; sem credenciais ⇒ registrar "N/A com consentimento do usuário" | sim |
| T13 | **App real**: colar a URL da playlist Deezer de T10 ⇒ Coleção mostra casamentos ⇒ baixar 1 faixa ⇒ concluída na Atividade | WebdriverIO |

## Portão
`npm run verify` ✔ · `npm run e2e` ✔ · `npm run verify:net` ✔ · `npm run e2e:app` ✔

## Armadilhas
- Deezer: use a busca **simples**; a avançada combinada retorna vazio (E1).
- `/album/{id}` não traz ISRC nas faixas — use `/album/{id}/tracks`.
- Nunca embutir credenciais do Spotify de terceiros; o recurso funciona sem Spotify (Deezer).
- Discografias grandes: paginar e limitar o paralelismo para não estourar o limite do Deezer
  (50 req / 5 s).

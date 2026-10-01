# E3 — Importação de playlists/álbuns de outros serviços

Objetivo: o usuário cola um link de playlist/álbum do **Deezer** ou do **Spotify**; o Reverb
lê a lista de faixas (com ISRC quando houver), encontra cada uma no YouTube Music (E1) e baixa
— uma vez ou como **sincronização** (F11).

## Deezer (sem chave — validado 2026-10-01)
- URLs: `deezer.com/<lang>/playlist/<id>`, `deezer.com/<lang>/album/<id>`,
  `deezer.com/<lang>/artist/<id>` (ver E5), links curtos `link.deezer.com/s/...` (seguir
  redirecionamento HTTP para obter a URL longa).
- Playlist: `GET https://api.deezer.com/playlist/{id}` ⇒ `title`, `nb_tracks`, `picture_xl`,
  `tracks.data[]`; listagem paginada `GET /playlist/{id}/tracks?index=<n>&limit=<m>` com `next`,
  `total` e `checksum`. Cada faixa: `id, title, title_version, duration, isrc, artist{name},
  album{title, cover_xl}, explicit_lyrics`.
- Álbum: `GET /album/{id}` (título, `release_date`, `upc`, `genres`, `cover_xl`, `nb_tracks`) +
  `GET /album/{id}/tracks` (faixas com `isrc`, `track_position`, `disk_number`).
- Limite: ~50 requisições a cada 5 s por IP — usar o limitador (5 req/s) da arquitetura.
- Erros: corpo `{"error":{"type":"DataException","code":800}}` = não encontrado (playlist
  privada/inexistente) ⇒ mensagem clara.

## Spotify (credenciais **do próprio usuário**)
- O usuário cria um app em `developer.spotify.com` e informa Client ID/Secret (já previstos em §6).
  **Nunca** embutir credenciais de terceiros (outras ferramentas distribuem credenciais próprias
  compartilhadas — não reutilizar).
- Fluxo Client Credentials: `POST https://accounts.spotify.com/api/token`
  (`grant_type=client_credentials`, Basic auth) ⇒ token de 1 h.
- Playlist: `GET https://api.spotify.com/v1/playlists/{id}/tracks?limit=100&offset=<n>`
  (campos úteis: `items[].track.{name, artists[].name, album.name, duration_ms, external_ids.isrc,
  album.images, track_number, disc_number}`); álbum: `GET /v1/albums/{id}` e `/v1/albums/{id}/tracks`.
- Restrições conhecidas da API para apps novos (desde fim de 2024): playlists **editoriais e
  algorítmicas** da própria Spotify (ex.: "Top 50", "Descobertas da Semana") podem retornar
  404/403. Tratar com mensagem: "Esta playlist é da Spotify e a API não permite lê-la; use uma
  playlist pública criada por um usuário". Playlists públicas de usuários funcionam.
- Retry: 429 respeitando `Retry-After`; 5xx com backoff.
- Sem credenciais ⇒ o botão de importar do Spotify explica como criar o app (link + passos).

## Comportamento comum
1. Lista de faixas normalizada: `{ title, artists[], album, duration_s, isrc?, track_no?,
   disc_no?, cover_url?, explicit? }`.
2. Para cada faixa: E1 (ISRC primeiro, depois texto). Mostrar na tela de Coleção o resultado do
   casamento por faixa: ✓ encontrada (com confiança), ⚠ duvidosa (para revisar), ✗ não encontrada.
3. Os metadados da **fonte importada** (título, artistas, álbum, faixa, capa) têm prioridade sobre
   a identificação (equivalem a um `metadata_override` vindo da fonte), porque a fonte é confiável.
4. Sincronização: a sync guarda o provedor de origem (`deezer`/`spotify`) e o ID da playlist;
   o diff (F11) usa o ID da faixa **na fonte** como chave; o vínculo com o vídeo do YouTube fica
   em `sync_items`.

## Testes de aceitação
- Parsers com respostas gravadas (Deezer playlist/álbum/erro 800; Spotify token/playlist/404/429).
- Classificação de URLs Deezer/Spotify (inclui link curto com redirecionamento — wiremock).
- Rede: importar a playlist Deezer obtida de `GET https://api.deezer.com/chart/0/playlists?limit=1`
  (pública, muda com o tempo) ⇒ lista com `isrc` em todas as faixas; casar as 3 primeiras via E1
  com confiança ≥ 0.85 em pelo menos 2.

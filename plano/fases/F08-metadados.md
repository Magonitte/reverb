# F08 — Identificação de metadados com nota de confiança

**Objetivo:** cada download sai com artista/título/álbum/ano corretos sempre que possível, sem
"chutes" errados: faixa oficial do YouTube Music quando existir, provedores sem chave (Deezer,
iTunes, MusicBrainz), pontuação de confiança, fila de revisão para casos duvidosos e detecção
de conteúdo não-musical.

**Pré-requisito:** F07. **Arquitetura:** §11 inteiro, §5 (`metadata_cache`), §15.

## Tarefas

1. `metadata::normalize` e `metadata::score` exatamente como §11.2 (pesos, tolerâncias,
   penalidades, teto 0.84 sem duração).
2. `metadata::parse_title` (§11.3) — atenção: convenção "Artista - Título" (o antigo invertia).
3. `metadata::content_type` (§11 passo 1).
4. Provedores `deezer`, `itunes`, `musicbrainz` (+ capa CAA como URL candidata), cada um
   implementando `trait MetadataProvider { async fn search(&self, q: &Query) -> Vec<Candidate> }`.
   `Candidate { provider, provider_id, title, artists, album, album_artist?, year?, genre?,
   track_no?, track_total?, disc_no?, duration_s?, cover_url?, mb_recording_id? }`.
   Detalhes extras (Deezer `/track/{id}` e `/album/{id}` para faixa/gênero) só para o
   candidato vencedor. Limitador de taxa por provedor (§11 tabela) compartilhado no processo.
   Base URLs injetáveis; `User-Agent` de §11.
5. Cache (`metadata_cache`, 30 dias) envolvendo cada provedor.
6. **Versão oficial**: `find_official_version(video: &VideoInfo, isrc: Option<&str>) ->
   Option<OfficialMatch { video_id, url, score, title, artist, album, isrc, via: isrc|text }>`
   (§11 passo 3) seguindo o estudo **`plano/anexos/estudos/E1-matching-isrc.md`**: ISRC primeiro
   (busca `<ISRC>` no YouTube Music), depois texto com os **filtros eliminatórios** e a
   penalidade cumulativa de palavras de versão de E1, artistas múltiplos e desempate; detalhes
   dos 3 primeiros em paralelo; corte 0.80. Funções puras `is_valid_isrc`, `eliminatory_filters`,
   `version_word_penalty`, `multi_artist_sim`, `tie_break` (testáveis isoladamente).
   O ISRC do candidato Deezer (via `/track/{id}`) e o encontrado aqui são guardados no resultado
   (a F09 grava em `library.isrc`).
7. **Passos de metadados na fila** (pipeline passa a ser `resolve_source → download → convert →
   identify → move`): executam o pipeline §11; se
   `preferOfficialAudio` e houver versão oficial para um job que ainda não baixou, **troca a URL
   de origem** antes do download (para isso o passo `metadata` de "resolução de fonte" roda
   **antes** do download: dividir em `resolve_source` (antes do download) e `identify` (depois,
   usando a duração real do arquivo)). Resultado gravado no job em `metadata_result_json` e
   `confidence` (colunas já existentes desde a F01 — **não** usar `metadata_override_json`, que é
   exclusivo da edição do usuário) e repassado aos passos seguintes. Se a fonte for trocada pela
   versão oficial, atualizar também `source_id`/`source_url` do job. `offlineMode` ⇒ só base + parse.
8. `metadata_override` do usuário (vindo do Preview) tem **prioridade máxima** e pula a identificação.
9. Comandos: `find_official_version`, `metadata_preview(url | video_info) -> MetadataResult
   { fields, confidence, source, bucket: auto|review|none, candidates }` (simulação sem baixar —
   usada no Preview), `metadata_search(query) -> Vec<Candidate>` (paridade `search_metadata`).
10. **UI**: no Preview, card "Versão oficial disponível" (toggle pré-marcado, mostra título/álbum
    e por quê: "áudio de estúdio, metadados completos"); botão "Pré-visualizar metadados" ⇒
    painel com campos, % de confiança, fonte, e **edição** antes de baixar (vira
    `metadata_override`). Badge de confiança nos itens concluídos da Atividade.
11. Gravar respostas HTTP reais (Deezer, iTunes, MusicBrainz) das consultas de FX2, FX3 e FX1
    ("Me at the zoo" / "jawed") em `tests/fixtures/http/`.

## Testes de verificação

| # | Teste | Rede |
|---|-------|------|
| T1 | `normalize`: ≥ 15 casos (acentos, "(Official Video)", "[4K]", "feat.", "&", "(Ao Vivo)" mantido, aspas) | não |
| T2 | `score` — tabela ≥ 20 pares com faixa esperada de resultado, incluindo: título/artista/duração idênticos ⇒ ≥ 0.95; mesmo título, artista diferente ⇒ < 0.85; "Song" × "Song (Live)" ⇒ penalizado; sem duração ⇒ ≤ 0.84; clipe com Δ 30 s na tolerância de clipe ⇒ dur_score > 0; **"Me at the zoo"/"jawed"/19 s × "At the Zoo"/"Deborah Lurie"/120 s (duração sintética) ⇒ < 0.85, ou seja, nunca aplicado automaticamente** (o erro real do app antigo; além disso o `content_type = other` impede a consulta — T8) | não |
| T3 | `parse_title` ≥ 25 casos: "Artista - Título", "Artista – Título (Official Video)", "Artista | Título", "Título" sem separador (artista = canal sem " - Topic"/"VEVO"), "A - B feat. C", aspas, "Artista - Título (Ao Vivo)" mantém "(Ao Vivo)" | não |
| T4 | `content_type`: FX1 ⇒ other; FX2 ⇒ music; FX3 ⇒ music (categoria Music); canal "X - Topic" ⇒ music | não |
| T5 | Provedores com wiremock + respostas gravadas: parsing correto de candidatos de cada provedor; erro HTTP 500/timeout ⇒ lista vazia + log, sem derrubar o pipeline | não |
| T6 | Limite de taxa MusicBrainz: 5 consultas seguidas levam ≥ 4 s (tempo pausado do tokio) | não |
| T7 | Cache: 2ª consulta igual não chama o servidor (contador do wiremock); expirado (>30 dias, relógio injetado) chama | não |
| T8 | Pipeline com fixtures: FX2 ⇒ fonte `youtube_music`, confiança 1.0, álbum "Whenever You Need Somebody", ano 1987; FX1 ⇒ `other`, **nenhuma** requisição aos provedores; caso sintético de confiança 0.7 ⇒ `needs_review` com candidatos; `offlineMode` ⇒ zero requisições | não |
| T9 | Override do usuário vence tudo e pula provedores | não |
| T10 | Versão oficial com fixtures de FX3 + busca gravada ⇒ `lYBUbBu4W08` com score ≥ 0.80; vídeo sem correspondência ⇒ `None` | não |
| T10b | E1 (puro): `is_valid_isrc` ≥ 8 casos; filtros eliminatórios ≥ 12 casos; penalidade cumulativa (resultado com "live" e "remix" ausentes na busca ⇒ −0.30 no `title_sim`); artistas múltiplos ≥ 5 casos; desempate (oficial vence vídeo com score até 0.08 maior) | não |
| T10c | Provedor Deezer usa a **busca simples** (`q=<artista> <título>`) — teste com wiremock que falha se a query contiver `artist:"` combinado com `track:"` | não |
| T10d | (rede) E1: busca `GBARL9300135` no YouTube Music ⇒ 1º `lYBUbBu4W08`; `GBARL0600786` ⇒ resultado ≠ `lYBUbBu4W08`; Deezer `track/isrc:GBARL9300135` ⇒ título "Never Gonna Give You Up"; busca simples Deezer "rick astley never gonna give you up" ⇒ 1º resultado do artista "Rick Astley" com `isrc` não vazio via `/track/{id}` | sim |
| T11 | UI: card de versão oficial (toggle altera a URL enviada no `enqueue`), painel de pré-visualização com edição gera `metadata_override` | Vitest |
| T12 | E2E mock: Preview de FX3 mostra sugestão oficial; pré-visualizar ⇒ editar artista ⇒ baixar ⇒ job leva o override | Playwright |
| T13 | (rede) `metadata_preview(FX3)` ⇒ oficial `lYBUbBu4W08`; resultado final artista "Rick Astley", título "Never Gonna Give You Up", confiança ≥ 0.85 | sim |
| T14 | (rede) `metadata_preview(FX1)` ⇒ `other`, sem candidatos | sim |
| T15 | (rede) Job real de FX3 com `preferOfficialAudio` ⇒ `source_id` final do job = `lYBUbBu4W08` (a faixa oficial, não o clipe `dQw4w9WgXcQ`) e metadados finais com álbum "Whenever You Need Somebody" e fonte `youtube_music`; com `preferOfficialAudio = false` ⇒ `source_id` permanece `dQw4w9WgXcQ` | sim |

## Portão
`npm run verify` ✔ · `npm run e2e` ✔ · `npm run verify:net` ✔ · `npm run e2e:app` ✔

## Armadilhas
- MusicBrainz bloqueia clientes sem `User-Agent` identificável ou acima de 1 req/s.
- iTunes Search devolve `artworkUrl100`; troque o sufixo para obter 1200 px.
- A duração real do arquivo (ffprobe) é mais confiável que a do YouTube para a identificação final.

# F09 — Capas, letras, ReplayGain, tags e organização da biblioteca

**Objetivo:** cada arquivo final sai com tags completas, capa quadrada de alta resolução, letra
(sincronizada quando houver), ReplayGain, no lugar certo da pasta de música e registrado na
biblioteca. Nenhum arquivo intermediário sobra.

**Pré-requisito:** F08. **Arquitetura:** §12, §13, §5 (`library`), §6 (configs F09).

## Tarefas

1. `artwork`: cadeia de candidatos (§12), download com limite de 15 MB, decodificação, recorte
   central se aspecto > 1.05, redimensionamento ≤ 1200 px, JPEG q90; resultado em memória +
   `cover_source`. Falha em todos ⇒ segue sem capa (não falha o job).
2. `lyrics`: cliente LRCLIB (§11 tabela) com `get` e fallback `search` (§12); resultado
   `{ plain?, synced? }`. LRCLIB é a **única** fonte (decisão do estudo
   `plano/anexos/estudos/E6-letras-decisao.md`); se a letra sincronizada vier no formato LRC
   "enhanced" (marcas `<mm:ss.xx>` por palavra), preservá-la intacta no `.lrc` e no tag.
3. `loudness`: análise EBU R128 via ffmpeg (§12), parse robusto do resumo (procure o bloco
   "Summary:"), cálculo dos valores de ReplayGain e R128.
4. `tagging`: `write_tags(path, &TrackTags)` e `read_tags(path) -> TrackTags` com lofty para
   mp3/m4a/opus/flac (§12); gravação + releitura de verificação. `TrackTags` com `TS`
   (usado também pelo editor na F10).
5. `organize::template`: `render(template, &TrackTags, ctx) -> PathBuf` com todas as regras de
   §13 (variáveis, vazios, `other`, `autoOrganize=false`, sanitização, limites, colisão) e
   `move_into_library` atômico (§12).
6. `library` (repo): `insert_from_job` (grava também `isrc` quando o resultado da identificação
   tiver — F08/E1), `get`, `find_by_source` (usado pelas duplicatas da F04), `find_by_isrc`
   (usado pela F15).
7. **Pipeline da fila** final: `resolve_source → download → convert → identify → artwork →
   lyrics → loudness → tagging → organize`. Cada passo respeita as opções do job e as
   configurações (`fetchArtwork`, `fetchLyrics`, `writeLrcFile`, `writeFolderCover`,
   `normalizeVolume`, `autoOrganize`, `offlineMode`). Falhas de capa/letra/loudness geram
   aviso no job (coluna `warnings_json`, já existente desde a F01; chaves i18n) mas **não**
   falham o job; falha de tags ou de mover **falha** o job.
   Pasta raiz do `organize`: `job.options.output_dir` ⇒ senão `sync.output_dir` (se o job vier
   de uma sincronização) ⇒ senão `settings.outputDir` resolvido (§4).
8. `.lrc` e `cover.jpg` são movidos/escritos junto com o áudio (mesmo nome base para o `.lrc`).
9. UI: item concluído na Atividade mostra miniatura da capa embutida (comando
   `library_cover(id) -> data URL` pequeno, 256 px), avisos, e "Abrir pasta"/"Abrir arquivo".
   Configurações › Downloads: modelo de nomes com **prévia ao vivo** (comando
   `template_preview(template) -> exemplo`), e › Metadados com os toggles desta fase.

## Testes de verificação

| # | Teste | Rede |
|---|-------|------|
| T1 | Capa: imagem sintética 1280×720 com quadrado vermelho central ⇒ saída 720×720 (ou ≤1200), pixel central vermelho, JPEG válido; imagem 3000×3000 ⇒ 1200×1200; dado inválido ⇒ próximo candidato | não |
| T2 | LRCLIB com wiremock: encontrado com sincronizada; só simples; 404 no `get` ⇒ `search` escolhe menor Δ duração; nada ⇒ `None` | não |
| T3 | Loudness: (a) parser com um resumo real do `ebur128` gravado como fixture (extrair I e Peak exatos); (b) gerar ruído rosa e o mesmo ruído com `volume=6dB` ⇒ o ganho calculado do segundo é 6 ±0.5 dB menor que o do primeiro; (c) strings de tag no formato `"-x.xx dB"`, pico com 6 casas, `R128_TRACK_GAIN` inteiro coerente com a fórmula | não |
| T4 | Tags: para cada formato (mp3, m4a, opus, flac) gerado por ffmpeg ⇒ gravar todos os campos + capa + letra + ReplayGain ⇒ reler com lofty **e** conferir com `ffprobe -show_format` (title/artist/album) | não |
| T5 | Modelo de nomes (comparar por **componentes** do caminho, não por string, para valer em Windows e Linux): tabela ≥ 15 casos (todas as regras de §13: vazio de faixa colapsa " - ", álbum vazio ⇒ "Singles", artista vazio ⇒ texto do idioma, `other` ⇒ `Outros/<canal>/<título>`, `autoOrganize=false`, reservados, 300 caracteres, colisão ⇒ " (2)") | não |
| T6 | Mover entre volumes (forçar o caminho copiar+remover por parâmetro de teste) preserva conteúdo e remove a origem | não |
| T7 | Pipeline completo com `FakeBackend` que "baixa" um arquivo opus sintético + provedores wiremock ⇒ arquivo no caminho do modelo, tags/capa/letra/RG presentes, `.lrc` e `cover.jpg` criados, linha na `library` e na busca FTS, `tmp/` vazio | não |
| T8 | Falha de capa/letra ⇒ job `done` com avisos; falha de escrita de tag (arquivo somente leitura) ⇒ job `failed` com `disk`/mensagem clara e sem arquivo parcial no destino | não |
| T9 | Duplicata agora também detectada pela `library` (F04 T12 com biblioteca preenchida) | não |
| T10 | UI: prévia ao vivo do modelo; item concluído mostra capa e avisos | Vitest + Playwright |
| T11 | (rede) FX2 com configuração padrão ⇒ caminho contém `Rick Astley` e `Whenever You Need Somebody` e termina em `Never Gonna Give You Up.opus`; capa embutida quadrada ≥ 500 px; letra presente; tags de ReplayGain presentes; `.lrc` existe; registro na biblioteca | sim |
| T12 | (rede) FX1 ⇒ `Outros/jawed/Me at the zoo.opus`, sem consulta de letras, sem `.lrc` | sim |
| T13 | **App real**: baixar FX2 pela UI ⇒ item concluído mostra capa; "Abrir pasta" não dá erro | WebdriverIO |

## Portão
`npm run verify` ✔ · `npm run e2e` ✔ · `npm run verify:net` ✔ · `npm run e2e:app` ✔

## Armadilhas
- Opus/Ogg: capa vai em `METADATA_BLOCK_PICTURE` (lofty cuida, mas confira na releitura).
- MP4: ano em `©day`, faixa em `trkn` — use os `ItemKey` genéricos do lofty e verifique.
- ReplayGain no MP3 vai em frames `TXXX` — confira que o lofty grava via `ItemKey::ReplayGainTrackGain`.
- Nunca deixe o arquivo final parcialmente escrito: tags são gravadas **no tmp**, antes de mover.

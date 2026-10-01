# F03 — Motor de download (yt-dlp + conversão)

**Objetivo:** analisar, buscar, baixar e converter pela linha de comando (`reverb-cli`), com
progresso estruturado, cancelamento, pasta temporária limpa e erros classificados.

**Pré-requisito:** F02. **Arquitetura:** §7, §8, §9 (classificação), §13 (sanitização), §14, §19.
**Anexos:** `ytdlp-contrato.md`, `fixtures.md`.

## Tarefas

1. `reverb-core::urlkind`: `classify(&str) -> UrlKind` (§14) — função pura. Normalizar
   `youtu.be/ID` e `music.youtube.com/watch?v=ID` para `source_id = ID`.
2. `reverb-core::ytdlp::args`: construtores **puros** de argumentos para analisar (vídeo/coleção),
   buscar (ytmusic/youtube) e baixar (§8), recebendo um `YtDlpContext { ytdlp_path, js_runtime_arg,
   ffmpeg_dir, cookies: Option<CookiesArg>, limit_rate_mbps: Option<f64>, pot_args: Option<Vec<String>> }`
   (`pot_args` vem do `PotServer` da F02 quando o provedor de PO token está ativo) e as opções do job
   (`sponsorblock: Option<Vec<String>>`, `tmp_dir`).
3. `reverb-core::ytdlp::parse`: parser de linhas `REVERB_PROGRESS`/`REVERB_DONE` (§8 + anexo) ⇒
   `ProgressUpdate { downloaded, total: Option, speed: Option, eta: Option, finished: bool }`
   e `DoneInfo { id, title, filepath, ext, abr, acodec, format_id, duration }`; modelos
   `VideoInfo` (campos §8, `audio_formats` filtrados conforme o anexo, `best_audio_abr`,
   `is_official_track`, `chapters`), `CollectionInfo`, `SearchResult`. Todos com `TS`.
4. `reverb-core::ytdlp::errors`: `classify_stderr(&[String]) -> ErrorKind` (§9).
5. `reverb-core::ytdlp::runner`: `YtDlpRunner` executa via `spawn_tool` (F02) segurando o guard de
   leitura das ferramentas, lê stdout/stderr por linha (UTF-8 com perdas toleradas), watchdog de
   300 s, `CancellationToken`, guarda as últimas 200 linhas do stderr, devolve
   `Result<DoneInfo, DownloadError { kind, message, stderr_tail }>`. Funções `analyze`,
   `search`, `download`. Timeouts de §8.
6. `reverb-core::transcode`: conversão por perfil (§7) com progresso (`-progress pipe:1`),
   cancelamento; `probe(path)` via `ffprobe -v error -print_format json -show_format -show_streams`
   ⇒ `{ codec, bitrate_kbps, duration_s, sample_rate, channels }`.
7. `reverb-core::profiles`: tabela de perfis (§7) com id, chave i18n, extensão, args, flag
   `reencodes`. `profile_for_source(original_ext)` para o `original`.
8. `reverb-core::workspace`: `JobWorkspace::new(data_dir, job_id)` cria `tmp/<job_id>/` e
   apaga no `Drop`; `sweep_orphans(data_dir)` na inicialização.
9. `reverb-core::organize::sanitize`: `sanitize_component`, `sanitize_path` (§13 regra 4) e
   `unique_path` (sufixo ` (2)`). (O renderizador de modelos vem na F09.)
10. `trait DownloadBackend` (`analyze`, `download(job_spec, progress_cb, cancel) -> DoneInfo`),
    implementação `YtDlpProcessBackend` (usa runner). Mantido genérico para o Android (A1).
11. CLI: `reverb-cli analyze <url> [--json]`, `reverb-cli search <texto> [--source ytmusic|youtube]`,
    `reverb-cli download <url> --profile <id> --out <dir>` (baixa ⇒ converte ⇒ move para
    `<out>/<titulo sanitizado>.<ext>`; imprime progresso por linha e o caminho final).
12. Gravar fixtures: `-J` de FX1, FX2, FX3 e `--flat-playlist -J` de FX4 em
    `tests/fixtures/ytdlp/` **removendo** as URLs dos formatos (campo `url`, `manifest_url`,
    `fragments`) para não versionar URLs assinadas; e um stderr real de erro "Video unavailable"
    (usar um ID inexistente, ex. `https://www.youtube.com/watch?v=aaaaaaaaaaa`).

## Testes de verificação

| # | Teste | Rede |
|---|-------|------|
| T1 | `classify`: toda a tabela do anexo de fixtures (válidas, coleção, busca, inválidas) — ≥ 25 casos | não |
| T2 | Snapshot (`insta`) dos argumentos de: analisar vídeo, analisar coleção, busca ytmusic, busca youtube, download com e sem SponsorBlock, com cookies de navegador, com cookies de arquivo, com limite de velocidade, com `pot_args`, com caminhos contendo espaços e acentos (`C:\Users\Jean Carlos de Souza\Música Teste ç`) — confirmar que cada caminho é **um único argumento**. Para o snapshot ser igual no Windows e no Linux (CI), normalizar `\` ⇒ `/` na representação do snapshot | não |
| T3 | Parser de progresso com as 4 linhas reais do anexo + linha sem `total_bytes` + linha com `eta`/`speed` nulos + linha lixo (ignorada) + título com emoji/acentos no `REVERB_DONE` | não |
| T4 | Modelos: parse das fixtures FX1/FX2/FX3/FX4 ⇒ FX2 `is_official_track = true`, album correto; FX1 sem `track`; FX4 10 entradas com duração; `audio_formats` sem storyboards | não |
| T5 | `classify_stderr` — ≥ 18 casos cobrindo todas as linhas da tabela §9, incluindo o stderr real gravado ("unavailable") | não |
| T6 | Runner com `fake-tool --stdout-lines <arquivo com linhas reais>` ⇒ callbacks de progresso na ordem e `DoneInfo` correto | não |
| T7 | Runner: `fake-tool --exit 1 --stderr "ERROR: … Video unavailable"` ⇒ `DownloadError{kind: unavailable}` | não |
| T8 | Cancelamento: runner com `fake-tool --spawn-child-sleep 60` cancelado ⇒ retorna `cancelled` em ≤ 3 s, árvore morta, workspace apagado | não |
| T9 | Watchdog (com tempo reduzido por parâmetro de teste, ex. 2 s) mata processo silencioso ⇒ `network` | não |
| T10 | Transcode (ffmpeg real de `.test-tools`): gerar 10 s de **ruído rosa estéreo** (`-f lavfi -i anoisesrc=color=pink:amplitude=0.3:duration=10 -ac 2 -c:a libopus -b:a 160k`; ruído, não seno, para que codificadores VBR produzam bitrate realista) ⇒ converter para cada perfil ⇒ `probe` confere codec (`mp3`, `aac`, `opus`, `flac`), bitrate (mp3_320 = 320 ±10; mp3_v0 entre 180 e 330; opus_96 entre 70 e 115; aac_256 entre 180 e 300) e duração 10 ±0.2 s; progresso chega a 100 % | não |
| T11 | Sanitização: tabela ≥ 15 casos (`CON`, `aux.txt`, `a:b`, `a?b*`, `nome.` , `nome `, emoji, 300 caracteres, combinação de acentos NFD, barra invertida) e `unique_path` | não |
| T12 | `sweep_orphans` remove pastas antigas de `tmp/` | não |
| T13 | (rede) `reverb-cli download FX1 --profile original --out "<tmp>/Música Teste ç"` ⇒ existe 1 arquivo `.opus`, duração 19 ±1 s (ffprobe), tamanho > 100 KB, `tmp/` vazio | sim |
| T14 | (rede) mesmo com `--profile mp3_v0` ⇒ `.mp3`, codec mp3 | sim |
| T15 | (rede) `analyze FX2` ⇒ track/artist/album de `fixtures.md`; `analyze FX4` ⇒ 10 entradas; `analyze FX5` ⇒ 10 entradas | sim |
| T16 | (rede) `search "rick astley never gonna give you up" --source ytmusic` ⇒ 1º id `lYBUbBu4W08` | sim |
| T17 | (rede) URL inexistente ⇒ erro `unavailable` | sim |

## Portão
`npm run verify` ✔ · `npm run e2e` ✔ · `npm run verify:net` (T13–T17) ✔

## Armadilhas
- Não use `--print` sem `after_move:` — ele implica `--simulate`.
- `-x --audio-format best` não recodifica; quem recodifica é **nosso** transcode.
- No Windows, o stdout do yt-dlp pode vir em blocos parciais: leia por linha com buffer.
- Defina `PYTHONIOENCODING=utf-8` e `PYTHONUTF8=1` no ambiente do processo do yt-dlp para
  garantir UTF-8 no Windows (o executável é Python empacotado).

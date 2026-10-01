# 01 — Arquitetura

Este documento é a **fonte da verdade técnica**. As fases referenciam suas seções (§N).

---

## §1. Visão geral

```
┌──────────────────────────── App Tauri 2 (src-tauri) ─────────────────────────────┐
│  Frontend React 19 + TS (WebView)                                                 │
│   └─ só invoke()/listen()/controles de janela                                     │
│                         │ IPC (comandos + eventos)                                │
│  Camada Tauri fina: comandos, eventos, plugins (updater, tray, deep-link…)        │
│                         │                                                         │
│  crates/reverb-core (Rust puro, SEM dependência de Tauri)                         │
│   ├─ db (SQLite)        ├─ settings      ├─ tools (yt-dlp/deno/ffmpeg/fpcalc)     │
│   ├─ ytdlp (runner)     ├─ transcode     ├─ queue (scheduler + autocura)          │
│   ├─ metadata (+score)  ├─ artwork/lyrics/loudness/tagging/organize               │
│   ├─ library/sync       ├─ providers (soundcloud/bandcamp/archive/jamendo)        │
│   └─ events (trait EventSink)                                                     │
└───────────────────────────────────────────────────────────────────────────────────┘
crates/reverb-cli  → binário de linha de comando que usa o mesmo core (testes e diagnóstico)
crates/fake-tool   → binário de teste que simula ferramentas externas (versões, sleep, filhos)

Ferramentas externas (baixadas e atualizadas pelo próprio app, nunca embutidas no instalador):
  yt-dlp (executável oficial standalone, já inclui yt-dlp-ejs) · Deno (runtime JS exigido pelo
  YouTube) · FFmpeg/ffprobe (builds do yt-dlp) · fpcalc (Chromaprint, opcional) ·
  bgutil (plugin + servidor de PO token, roda no Deno; ligado sob demanda — estudo E4)
```

**Sala limpa:** funcionalidades inspiradas em outros projetos são implementadas **somente** a
partir das especificações em `plano/anexos/estudos/` (E1–E7). O código-fonte de origem nunca é
consultado nem copiado.

**Por que assim** (decisões já tomadas — não reabrir):
- Sem Python: elimina sidecar, empacotamento Nuitka/PyInstaller e o protocolo NDJSON próprio.
- yt-dlp como executável gerenciado: atualizável sem lançar versão nova do app (a causa da
  quebra do projeto antigo foi yt-dlp desatualizado + falta de runtime JS).
- `reverb-core` sem Tauri: testável com `cargo test` puro, reutilizável no CLI e no Android.
- Conversões feitas pelo **nosso** passo de ffmpeg (não pelo yt-dlp): controle total, progresso,
  testável offline. O yt-dlp só baixa o áudio original (remuxado, sem recodificar).
- Tags escritas pelo **nosso** código (crate `lofty`), não `--embed-metadata`, pois os
  metadados finais vêm do nosso pipeline de identificação.

## §2. Stack e versões

| Camada | Tecnologia | Regra de versão |
|--------|-----------|-----------------|
| Desktop | Tauri **2.x** (`tauri = "2"`, CLI `@tauri-apps/cli@^2`) | **Nunca 3.x alpha** |
| Frontend | React 19, TypeScript (ver protocolo §7.2), Vite (última estável) | última estável |
| Roteamento | `react-router` (última estável; modo declarativo, `HashRouter`) | consultar docs da versão |
| Estado | Zustand 5 | |
| CSS | Tailwind CSS 4 via `@tailwindcss/vite` (tokens em `@theme`, sem `tailwind.config`) | |
| Animação | `motion` (`import { motion } from "motion/react"`), respeitando `prefers-reduced-motion` | |
| Ícones | `lucide-react` | |
| i18n | `i18next` + `react-i18next` | |
| Listas grandes | `@tanstack/react-virtual` | |
| Arrastar/reordenar | `@dnd-kit/core` + `@dnd-kit/sortable` | |
| Testes UI | Vitest + Testing Library + jsdom; `@tauri-apps/api/mocks` (`mockIPC`) | |
| E2E | Playwright (`@playwright/test`) + `@axe-core/playwright` | |
| E2E app real | `tauri-driver` + WebdriverIO (F07) | |
| Rust async | `tokio` (full), `tokio-util` (CancellationToken) | |
| Banco | `rusqlite` com feature `bundled` (inclui FTS5) | |
| HTTP | `reqwest` (features `json`, `stream`, `rustls-tls`; sem default-tls) | |
| Processos | `process-wrap` com features `tokio1`, `job-object` (Win), `process-group` (Unix), `creation-flags`, `kill-on-drop` | |
| Tags | `lofty` | |
| Imagens | `image` (jpeg, png, webp) | |
| Similaridade | `strsim`, `unicode-normalization` | |
| Zip/hash | `zip` (última **estável**), `sha2`, `hex` | |
| Erros/log | `thiserror`, `anyhow` (só em bins), `tracing`, `tracing-subscriber`, `tracing-appender` | |
| Tipos TS | `ts-rs` (exporta para `src/bindings/`) | |
| CLI | `clap` (derive) | |
| Testes Rust | `tempfile`, `wiremock`, `sysinfo` (verificar processos mortos), `insta` (snapshots) | |
| Outros | `dirs`, `uuid` (v4), `chrono`, `regex`, `once_cell`/`std::sync::LazyLock`, `trash`, `notify` (+`notify-debouncer-mini`), `semver` | |

Plugins Tauri (todos `"2"`): `tauri-plugin-updater`, `tauri-plugin-process`,
`tauri-plugin-single-instance` (feature `deep-link`), `tauri-plugin-deep-link`,
`tauri-plugin-notification`, `tauri-plugin-autostart`, `tauri-plugin-global-shortcut`,
`tauri-plugin-clipboard-manager`, `tauri-plugin-dialog`, `tauri-plugin-opener`,
`tauri-plugin-window-state`, `tauri-plugin-os`, `tauri-plugin-log` (opcional; pode usar só tracing).
`tauri` com features `tray-icon`, `image-png`.
**Todos usados a partir do Rust** (protocolo §7.3). Pacotes npm de plugins **não** são
necessários, exceto `@tauri-apps/api` (core, event, window).

## §3. Estrutura de pastas do código

```
Reverb_claude/
├── Cargo.toml                    # workspace: members = crates/*, src-tauri ; resolver = "2"
├── .cargo/config.toml            # [env] TS_RS_EXPORT_DIR = { value = "src/bindings", relative = true }
├── package.json                  # scripts verify, e2e, etc.
├── vite.config.ts  tsconfig*.json  eslint.config.js  playwright.config.ts  vitest.config.ts
├── index.html
├── src/
│   ├── main.tsx  App.tsx  routes.tsx
│   ├── bindings/                 # GERADO por ts-rs (não editar à mão; commitado)
│   ├── lib/
│   │   ├── ipc/                  # api.ts (wrappers tipados), events.ts, isTauri.ts
│   │   │   └── mock/             # backend falso p/ navegador, Vitest e Playwright
│   │   ├── i18n.ts  format.ts  shortcuts.ts  urlKind.ts
│   ├── stores/                   # zustand: settings, jobs, library, syncs, tools, updater, ui
│   ├── components/
│   │   ├── ui/                   # primitivos (Button, Dialog, Toggle, …)
│   │   ├── layout/               # Titlebar, Sidebar, BottomNav, ScreenOutlet
│   │   └── <domínio>/            # command-bar, preview, collection, activity, library, review, tag-editor, playlists, settings, onboarding
│   ├── routes/                   # uma tela por arquivo (lazy)
│   ├── locales/{pt-BR,en}/translation.json
│   └── styles/ tokens.css  tailwind.css  base.css
├── crates/
│   ├── reverb-core/  (src/{lib.rs, paths.rs, error.rs, events.rs, db/, settings.rs, tools/, ytdlp/, transcode.rs, queue/, metadata/, artwork.rs, lyrics.rs, loudness.rs, tagging.rs, organize.rs, library.rs, sync.rs, providers/, verify_lossless.rs, pot.rs (E4), import/ (F15), artists.rs (F15)}; migrations/*.sql; tests/)
│   ├── reverb-cli/
│   └── fake-tool/
├── src-tauri/  (src/{main.rs, lib.rs, commands/*.rs, tray.rs, updater.rs, integration.rs, headless.rs}; capabilities/default.json; tauri.conf.json; tauri.windows.conf.json; tauri.linux.conf.json; icons/)
├── tests/
│   ├── e2e/                      # Playwright (mock)
│   ├── e2e-app/                  # WebdriverIO + tauri-driver (app real)
│   └── fixtures/                 # JSON do yt-dlp gravados, respostas HTTP gravadas, áudios sintéticos gerados em runtime
├── scripts/                      # verify.mjs, check-i18n.mjs, scan-secrets.mjs, release.mjs, bump-version.mjs, test-prepare.mjs
├── docs/                         # README de uso, TROUBLESHOOTING, CHANGELOG
└── .github/workflows/            # ci.yml, release.yml
```

## §4. Diretórios de dados

- **Normal:** `app_data_dir` do Tauri (Windows: `%APPDATA%\com.reverb.desktop`; Linux:
  `~/.local/share/com.reverb.desktop`). No core, a função recebe o caminho já resolvido.
- **Portátil** (paridade): se existir `portable.txt` ao lado do executável → `<exe_dir>/data`.
- Layout: `reverb.db`, `logs/`, `tools/`, `tmp/`, `cache/`, `backups/`.
- O CLI aceita as flags globais `--data-dir <path>` (padrão = mesmo diretório do app) e
  `--tools-dir <path>` (padrão = `<data>/tools`) para que testes possam usar/inspecionar estados isolados.
- **Somente em builds de debug** (`cfg!(debug_assertions)`), o app honra as variáveis
  `REVERB_DATA_DIR` e `REVERB_TOOLS_DIR` (usadas pelos testes E2E do app real). Em release
  elas são ignoradas.
- **Testes nunca escrevem nas pastas reais do usuário** (Músicas, AppData do app): sempre
  diretórios temporários (`tempfile`) ou `REVERB_DATA_DIR` + `outputDir` temporários.
- Pasta de música padrão: `dirs::audio_dir()/Reverb` (Windows: `Músicas\Reverb`;
  Linux: `XDG_MUSIC_DIR/Reverb`; fallback `~/Music/Reverb`).
- Identificador do app: **`com.reverb.desktop`** (não usar sufixo `.app`).

## §5. Banco de dados (SQLite)

Abertura: `PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;`.
Acesso: uma conexão de escrita protegida por `Mutex`, chamadas via `spawn_blocking`.
Migrações: arquivos `crates/reverb-core/migrations/NNNN_nome.sql` embutidos com `include_str!`,
aplicados em ordem dentro de transação, versão em `PRAGMA user_version`.

### `0001_init.sql`

```sql
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);         -- valor JSON
CREATE TABLE kv (key TEXT PRIMARY KEY, value TEXT NOT NULL);               -- estado interno (últimas verificações, autocura…)

CREATE TABLE syncs (
  id TEXT PRIMARY KEY, provider TEXT NOT NULL DEFAULT 'youtube',  -- youtube | deezer | spotify (F15)
  url TEXT NOT NULL UNIQUE,
  playlist_id TEXT, title TEXT NOT NULL, thumbnail TEXT, profile_id TEXT NOT NULL,
  output_dir TEXT, interval_hours INTEGER NOT NULL DEFAULT 24,   -- 0 = só manual
  max_items INTEGER,                                             -- NULL = todos
  remove_deleted INTEGER NOT NULL DEFAULT 0, write_m3u INTEGER NOT NULL DEFAULT 1,
  enabled INTEGER NOT NULL DEFAULT 1, last_sync_at INTEGER, last_result_json TEXT,
  created_at INTEGER NOT NULL
);

CREATE TABLE library (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  file_path TEXT NOT NULL UNIQUE, missing INTEGER NOT NULL DEFAULT 0,
  provider TEXT, source_id TEXT, source_url TEXT, isrc TEXT,
  title TEXT NOT NULL, artist TEXT, album TEXT, album_artist TEXT,
  track_no INTEGER, track_total INTEGER, disc_no INTEGER, year INTEGER, genre TEXT,
  duration_s REAL, codec TEXT, bitrate_kbps REAL, source_abr_kbps REAL, profile_id TEXT,
  content_type TEXT NOT NULL DEFAULT 'music',          -- music | other
  metadata_source TEXT, confidence REAL,
  needs_review INTEGER NOT NULL DEFAULT 0, review_candidates_json TEXT,
  has_lyrics INTEGER NOT NULL DEFAULT 0, has_synced_lyrics INTEGER NOT NULL DEFAULT 0,
  cover_source TEXT, acoustid_id TEXT, mb_recording_id TEXT, replaygain_db REAL,
  lossless_verdict TEXT,                               -- F14
  origin TEXT NOT NULL DEFAULT 'download',             -- download | import | scan
  added_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE INDEX library_source ON library(provider, source_id);
CREATE INDEX library_review ON library(needs_review);
CREATE INDEX library_isrc ON library(isrc);
CREATE VIRTUAL TABLE library_fts USING fts5(
  title, artist, album, content='library', content_rowid='id',
  tokenize = "unicode61 remove_diacritics 2"
);
-- triggers AFTER INSERT/UPDATE/DELETE em library mantendo library_fts (padrão "external content" do FTS5)

CREATE TABLE jobs (
  id TEXT PRIMARY KEY, kind TEXT NOT NULL,             -- single | playlist_item | upgrade | chapters
  provider TEXT NOT NULL DEFAULT 'youtube',
  source_url TEXT NOT NULL, source_id TEXT, title TEXT, artist TEXT, thumbnail TEXT, duration_s REAL,
  profile_id TEXT NOT NULL, options_json TEXT NOT NULL DEFAULT '{}',
  metadata_override_json TEXT,                         -- edição do USUÁRIO (prioridade máxima)
  metadata_result_json TEXT, confidence REAL,          -- resultado da IDENTIFICAÇÃO (F08)
  warnings_json TEXT NOT NULL DEFAULT '[]',            -- avisos não fatais (F09: capa/letra/loudness)
  playlist_ctx_json TEXT,
  sync_id TEXT REFERENCES syncs(id) ON DELETE SET NULL,
  status TEXT NOT NULL, stage TEXT NOT NULL,
  progress REAL NOT NULL DEFAULT 0, overall_progress REAL NOT NULL DEFAULT 0,
  speed_bps REAL, eta_s INTEGER,
  error_kind TEXT, error_message TEXT, attempts INTEGER NOT NULL DEFAULT 0,
  output_path TEXT, library_id INTEGER REFERENCES library(id) ON DELETE SET NULL,
  position INTEGER NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, finished_at INTEGER
);
CREATE INDEX jobs_status ON jobs(status, position);

CREATE TABLE sync_items (
  sync_id TEXT NOT NULL REFERENCES syncs(id) ON DELETE CASCADE,
  source_id TEXT NOT NULL,                             -- ID da faixa NA FONTE (vídeo do YouTube, ou faixa Deezer/Spotify — F15)
  position INTEGER NOT NULL, title TEXT,
  state TEXT NOT NULL,                                 -- present | removed
  matched_video_id TEXT, match_confidence REAL,        -- F15: vídeo do YouTube casado (fontes não-YouTube)
  meta_json TEXT,                                      -- F15: metadados da fonte (título, artistas, álbum, isrc, faixa, capa)
  job_id TEXT, library_id INTEGER, first_seen_at INTEGER NOT NULL, removed_at INTEGER,
  PRIMARY KEY (sync_id, source_id)
);

-- F15: artistas seguidos (estudo E5)
CREATE TABLE followed_artists (
  id TEXT PRIMARY KEY, provider TEXT NOT NULL DEFAULT 'deezer', provider_artist_id TEXT NOT NULL,
  name TEXT NOT NULL, picture TEXT,
  monitor_existing TEXT NOT NULL DEFAULT 'latest',     -- all | latest | none
  monitor_new TEXT NOT NULL DEFAULT 'notify',          -- all | notify | none
  types_json TEXT NOT NULL DEFAULT '["album","ep","single"]',
  exclude_variants INTEGER NOT NULL DEFAULT 1,         -- excluir ao vivo/remix/deluxe duplicada/coletânea
  profile_id TEXT NOT NULL, output_dir TEXT,
  last_check_at INTEGER, created_at INTEGER NOT NULL,
  UNIQUE (provider, provider_artist_id)
);
CREATE TABLE followed_releases (
  artist_id TEXT NOT NULL REFERENCES followed_artists(id) ON DELETE CASCADE,
  provider_album_id TEXT NOT NULL, title TEXT NOT NULL, record_type TEXT NOT NULL,
  release_date TEXT, cover TEXT, monitored INTEGER NOT NULL DEFAULT 0,
  tracks_json TEXT,                                    -- faixas da fonte: título, isrc, posição, disco, duração
  first_seen_at INTEGER NOT NULL,
  PRIMARY KEY (artist_id, provider_album_id)
);

CREATE TABLE metadata_cache (key TEXT PRIMARY KEY, value TEXT NOT NULL, created_at INTEGER NOT NULL);
```

Timestamps: segundos Unix (`i64`). IDs de job/sync: UUID v4 string.

## §6. Configurações

Armazenadas na tabela `settings` (uma linha por chave, valor JSON). O backend é a fonte da
verdade; o frontend lê com `settings_get` e altera com `settings_update(patch)`.
`localStorage` só para conveniências de UI (aba aberta, colunas). Struct interna `Settings`
(com segredos) e `SettingsView` enviada à UI (sem segredos + `secretsStatus`), ambas com
`#[serde(rename_all = "camelCase")]`; só `SettingsView`/`SettingsPatch` derivam `TS` (detalhes na F01).

| Chave | Tipo | Padrão | Validação | Fase |
|-------|------|--------|-----------|------|
| outputDir | string | "" (= pasta de música padrão §4) | absoluto ou vazio; criado se não existir | F01 |
| fileTemplate | string | `{albumartist}/{album}/{track:02} - {title}` | variáveis válidas (§13) | F09 |
| autoOrganize | bool | true | — (false ⇒ `{artist} - {title}`) | F09 |
| defaultProfile | string | `original` | id de perfil existente (§7) | F03 |
| parallelism | int | 2 | 1..=4 | F04 |
| speedLimitMbps | number (decimal) | 0 | 0..=1000, unidade **MB/s** (megabytes por segundo, como no app antigo); 0 = sem limite; vira `--limit-rate <valor>M` | F04 |
| queueLimit | int | 500 | 10..=5000 | F04 |
| maxAttempts | int | 3 | 1..=10 | F04 |
| fetchMetadata | bool | true | | F08 |
| extractTitleFromVideo | bool | true | | F08 |
| preferOfficialAudio | bool | true | | F08 |
| confidenceAutoApply | number | 0.85 | 0.5..=1.0, > confidenceReview | F08 |
| confidenceReview | number | 0.60 | 0.3..=0.95 | F08 |
| offlineMode | bool | false | | F08 |
| fetchArtwork | bool | true | | F09 |
| writeFolderCover | bool | true | | F09 |
| fetchLyrics | bool | true | | F09 |
| writeLrcFile | bool | true | | F09 |
| normalizeVolume | bool | true | (ReplayGain, não destrutivo) | F09 |
| sponsorblockRemove | bool | false | | F03 |
| sponsorblockCategories | string[] | ["music_offtopic"] | subconjunto de sponsor, selfpromo, interaction, intro, outro, preview, music_offtopic, filler | F03 |
| splitChapters | "ask"\|"always"\|"never" | "ask" | | F13 |
| trimSilence | bool | false | | F13 |
| playlistPacingSeconds | int | 3 | 0..=60 | F11 |
| watchLibrary | bool | true | | F10 |
| theme | "dark"\|"light"\|"system" | "dark" | | F05 |
| language | "pt-BR"\|"en" | "pt-BR" | | F05 |
| transparency | "auto"\|"full"\|"reduced" | "auto" (Linux ⇒ reduced) | | F05 |
| launchAtStartup | bool | false | | F12 |
| startMinimized | bool | true | | F12 |
| minimizeToTray | bool | true | | F12 |
| closeToTray | bool | true | | F12 |
| completionNotifications | bool | true | | F12 |
| clipboardWatch | bool | false | | F12 |
| globalShortcut | string | "" (desligado) | acelerador válido do plugin | F12 |
| weeklySelfTest | bool | true | | F12 |
| onboardingCompleted | bool | false | | F12 |
| cookiesSource | "none"\|"firefox"\|"chrome"\|"edge"\|"brave"\|"file" | "none" | | F13 |
| cookiesFile | string | "" | arquivo existente se source=file | F13 |
| ytdlpChannel | "stable"\|"nightly" | "stable" | | F02 |
| jsRuntime | "auto"\|"managed-deno"\|"system-deno"\|"system-node" | "auto" | | F02 |
| autoUpdateTools | bool | true | | F02 |
| potProvider | "auto"\|"always"\|"off" | "auto" | (E4) `auto` = ligado pela autocura por 24 h após bloqueio | F02/F04 |
| qualityTargetKbps | int | 0 | 0 (desligado), 160 ou 256 (E5) | F15 |
| autoUpgrade | bool | false | (E5) só age abaixo do alvo | F15 |
| artistCheckIntervalHours | int | 24 | 6..=168 | F15 |
| autoCheckAppUpdates | bool | true | | F06 |
| acoustidKey, spotifyClientId, spotifyClientSecret, discogsToken, jamendoClientId | string | "" | segredos: nunca logar; mascarar ao exibir | F13/F14 |
| verifyLosslessOnImport | bool | true | | F14 |

`settings_update` valida, persiste, emite `settings://changed` e aplica efeitos colaterais
(ex.: `parallelism` ajusta a fila em tempo real; `launchAtStartup` liga/desliga autostart).
Chaves desconhecidas no patch ⇒ erro.

## §7. Perfis de saída

O yt-dlp **sempre** baixa o original. O perfil decide se nosso passo de ffmpeg converte.

| id | Nome (i18n) | Resultado | Conversão (ffmpeg, depois do download) |
|----|-------------|-----------|----------------------------------------|
| `original` | Original (sem recodificar) — **padrão** | `.opus` ou `.m4a` conforme a fonte | nenhuma |
| `mp3_v0` | MP3 VBR V0 (compatível) | `.mp3` | `-c:a libmp3lame -q:a 0` |
| `mp3_320` | MP3 320 kbps | `.mp3` | `-c:a libmp3lame -b:a 320k` |
| `aac_256` | AAC 256 kbps (Apple) | `.m4a` | `-c:a aac -b:a 256k` |
| `opus_96` | Opus 96 kbps (celular) | `.opus` | `-c:a libopus -b:a 96k` |
| `flac` | FLAC (avançado) | `.flac` | `-c:a flac -compression_level 8` |

Argumentos comuns da conversão:
`-hide_banner -nostdin -y -i <in> -map 0:a:0 -vn -map_metadata -1 <codec args> -progress pipe:1 -nostats <out>`.
Progresso: ler `out_time_us=` do stdout e dividir pela duração.
A UI exibe um aviso nos perfis que recodificam ("converte um áudio já comprimido; perde um
pouco de qualidade") e no `flac` ("não aumenta a qualidade; só ocupa mais espaço").
Se o original não for `.opus`/`.m4a`/`.mp3`/`.ogg`/`.flac`/`.wav` (caso raro), remuxar para
`.mka` não é aceito: recodificar para Opus 160k e registrar aviso no log.

## §8. Contrato do yt-dlp

Validado na prática em 2026-10-01 (ver `anexos/ytdlp-contrato.md` para saídas reais).

### Argumentos fixos (toda chamada)
`--ignore-config --color never --js-runtimes <runtime>:<caminho> --ffmpeg-location <dir-do-ffmpeg>`
+ cookies (F13: `--cookies-from-browser <navegador>` ou `--cookies <arquivo>`)
+ `--limit-rate <N>M` se `speedLimitMbps > 0`.
**Não** usar `--no-warnings`: os avisos vão para o stderr e são úteis no log e na classificação de erros.

`<runtime>:<caminho>` vem do gerenciador de ferramentas (§16): `deno:<caminho>` (gerenciado ou
do sistema) ou `node:<caminho>`.

### Analisar (um vídeo)
`yt-dlp <fixos> -J --no-playlist -- <url>` → JSON único no stdout. Timeout 90 s.
Campos usados: `id, title, duration, channel, uploader, thumbnails[], thumbnail, categories,
track, artist, artists, creators, album, release_year, release_date, chapters, formats[]
(format_id, acodec, vcodec, abr, ext), webpage_url, extractor_key`.
`is_official_track = track.is_some() && (artists|artist).is_some()`.

### Analisar (coleção: playlist, álbum, canal)
`yt-dlp <fixos> -J --flat-playlist -- <url>` → JSON com `id, title, channel, entries[]`
(`id, title, duration, url`). Timeout 180 s. URLs `music.youtube.com/browse/MPREb_…` são
redirecionadas pelo yt-dlp para a playlist `OLAK5uy_…` (normal; gera um WARNING).

### Buscar
- YouTube Music (músicas): `-J --flat-playlist --playlist-end <n> -- "https://music.youtube.com/search?q=<urlencoded>#songs"` — entradas trazem `id, title, url` mas **sem duração**; para pontuar, buscar detalhes dos 3 primeiros com "Analisar (um vídeo)" em paralelo.
- YouTube: `-J --flat-playlist -- "ytsearch<n>:<consulta>"`.

### Baixar
```
yt-dlp <fixos> --no-playlist --newline --progress
  -f "bestaudio[format_id!*=-drc]/bestaudio"
  -x --audio-format best
  --progress-template "download:REVERB_PROGRESS %(progress)j"
  --print "after_move:REVERB_DONE %(.{id,title,filepath,ext,abr,acodec,format_id,duration})j"
  [--sponsorblock-remove <cats>]
  -o "<tmp_do_job>/%(id)s.%(ext)s"
  -- <url>
```
- `-x --audio-format best` mantém o codec (Opus em WebM vira `.opus`, AAC vira `.m4a`) — sem recodificar.
- stdout: linhas `REVERB_PROGRESS {json}` (campos `status, downloaded_bytes, total_bytes,
  total_bytes_estimate, speed, eta`) e uma linha final `REVERB_DONE {json}` com `filepath` final.
  Ignorar linhas que não comecem com esses prefixos.
- stderr: guardar as últimas 200 linhas (classificação de erro §9 e log).
- Código de saída ≠ 0 ⇒ erro; classificar pelo stderr.
- Watchdog: sem nenhuma saída por 300 s ⇒ matar a árvore e classificar como `network`.
- Cancelamento: `CancellationToken` ⇒ matar a árvore (Job Object/grupo) ⇒ status `cancelled`.

### Outras regras
- O processo do Windows é `yt-dlp.exe` standalone (PyInstaller, que cria processo filho) — por
  isso matar a **árvore** é obrigatório.
- Cada job usa sua pasta `<data>/tmp/<job_id>/`, apagada ao final (sucesso, falha ou cancelamento)
  por um guard com `Drop`; na inicialização, apagar pastas órfãs em `tmp/`.

## §9. Taxonomia de erros, retry e autocura

`ErrorKind` (classificado por regex no stderr, case-insensitive, primeira regra que casar):

| kind | Padrões (exemplos) | Ação |
|------|-------------------|------|
| `cancelled` | (cancelamento pelo usuário) | nenhuma |
| `unavailable` | `Video unavailable`, `Private video`, `This video has been removed`, `members-only`, `This video is not available`, `HTTP Error 404` | falha permanente |
| `age_restricted` | `Sign in to confirm your age`, `age-restricted` | permanente; sugerir cookies |
| `bot_check` | `Sign in to confirm you.re not a bot`, `confirm you are not a robot` | **autocura com PO token** (passo 0 abaixo) se `potProvider = auto`; se já estava ligado ⇒ permanente; sugerir cookies / esperar |
| `geo_blocked` | `not available in your country`, `geo restrict` | permanente |
| `disk` | `No space left`, `Permission denied`, `Access is denied`, `ENOSPC` | permanente |
| `ffmpeg` | `ffmpeg not found`, `ffprobe not found`, `Postprocessing: .*(Error|failed)` | verificar ferramentas; 1 retry |
| `extractor` | `Unable to extract`, `nsig extraction failed`, `Signature extraction failed`, `Requested format is not available`, `HTTP Error 403`, `jsc`, `challenge`, `Some formats may be missing`, `No video formats found` | **autocura** |
| `network` | `Unable to download webpage`, `timed out`, `Connection (reset|refused|aborted)`, `getaddrinfo failed`, `Temporary failure in name resolution`, `HTTP Error 5\d\d`, `IncompleteRead`, watchdog | retry com backoff |
| `unknown` | qualquer outra coisa | 1 retry |

Retry (`network`, `unknown`, `ffmpeg`): até `maxAttempts` tentativas no total, espera 5 s, 30 s, 120 s
(depois 120 s). Durante a espera o job fica `queued` com `stage = "waiting_retry"`.

**Autocura** (`HealCoordinator`, um por app, com mutex — vários jobs falhando ao mesmo tempo
disparam **uma única** autocura):
0. **PO token** (estudo E4): se a falha for `bot_check`, ou `extractor` com `HTTP Error 403`, e
   `potProvider = "auto"` com o provedor ainda desligado ⇒ ligar o provedor bgutil por 24 h
   (instalar se faltar), reenfileirar os jobs afetados sem contar tentativa e **parar aqui**.
   Só se a falha se repetir com o provedor ligado seguir para os passos 1–4.
1. Emitir `heal://state {stage:"checking"}`; pausar o início de novos jobs.
2. Se existir yt-dlp mais novo no canal atual ⇒ instalar (espera jobs em execução terminarem
   seu processo, máx. 10 min) ⇒ reenfileirar os jobs que falharam com `extractor` (sem contar tentativa).
3. Se já estava na última estável, ou se após o passo 2 o erro voltar ⇒ trocar para **nightly**
   (persistir `ytdlpChannel = "nightly"` e `kv.heal_switched_to_nightly = <data>`), instalar,
   reenfileirar e notificar o usuário ("Mudamos o yt-dlp para o canal nightly…").
4. Se ainda falhar ⇒ marcar os jobs `failed` com `extractor` e mensagem i18n
   `errors.extractorPersistent`; emitir `heal://state {stage:"failed"}`.
5. Autocura no máximo 1 vez por hora (kv `heal_last_at`).
6. Volta automática para stable: na verificação diária, se o usuário não escolheu nightly
   manualmente e existir stable com data ≥ à da nightly instalada ⇒ voltar para stable.

## §10. Fila

Estados: `queued → running → done | failed | cancelled`; `failed|cancelled → queued` (retry manual).
Fila pausada: nenhum job novo inicia; os em execução continuam.
Estágios (`stage`): `waiting`, `waiting_retry`, `analyzing`, `downloading`, `converting`,
`metadata`, `artwork`, `lyrics`, `loudness`, `tagging`, `moving`, `done`.
Progresso geral ponderado: downloading 0–70 %, converting 70–80 %, metadata…tagging 80–97 %,
moving 97–100 % (estágios ausentes redistribuem proporcionalmente).
Concorrência: contador próprio com limite dinâmico (`parallelism` muda em tempo real; reduzir
não interrompe jobs em execução, só impede novos).
Ordem: `position` crescente; "Baixar agora" insere com `position = min - 1`.
Eventos `job://updated` com **throttle de 250 ms por job** (sempre emitir transições de estado
imediatamente). Restauração na inicialização: `running ⇒ queued` (mesmas tentativas).

Pipeline do job como lista de passos (`trait Step { fn stage(); async fn run(&mut JobCtx) }`):
F04 = [download, convert, move]; F08 ⇒ [resolve_source, download, convert, identify, move];
F09 ⇒ [resolve_source, download, convert, identify, artwork, lyrics, loudness, tagging, organize]
(`move` vira `organize`); F13 pode inserir `trim_silence` antes de `loudness` e o fluxo de
capítulos (`kind=chapters`). `resolve_source` executa o "Analisar (um vídeo)" (§8) — reaproveita
o `VideoInfo` se o job já veio do Preview com ele — e decide a fonte final. Os estágios exibidos
na UI (`stage`) mapeiam: resolve_source ⇒ `analyzing`; identify ⇒ `metadata`;
trim_silence ⇒ `converting`; organize/move ⇒ `moving`. O passo de download usa
`trait DownloadBackend` (implementação desktop: `YtDlpProcessBackend`; Android implementará outra).

## §11. Metadados

### Pipeline
O passo `resolve_source` (antes do download) executa os itens 1–3 abaixo; o passo `identify`
(depois do download, usando a duração real medida com ffprobe) executa 1, 2 e 4–6.
1. `content_type`: `music` se qualquer: `track` presente; `categories` contém `Music`;
   canal termina com ` - Topic`; uploader contém `VEVO`; URL de `music.youtube.com`.
   Senão `other`. (`other` ⇒ pula provedores; tags: title = título do vídeo, artist = canal.)
2. **Base**: se `is_official_track` ⇒ campos oficiais (`track`, `artists`/`artist`, `album`,
   `release_year`) com `confidence = 1.0`, `metadata_source = "youtube_music"`.
   Senão, se `extractTitleFromVideo` ⇒ `parse_title(title, channel)` (§11.3).
3. **Versão oficial** (se `preferOfficialAudio`, música, não oficial) — seguir o estudo **E1**:
   (a) se houver ISRC conhecido (vindo de importação, da fonte ou de um candidato Deezer com
   score ≥ `confidenceAutoApply` obtido por uma busca rápida no Deezer pelo artista+título
   analisados), buscar `<ISRC>` no YouTube Music (`#songs`) e aplicar a regra de E1;
   (b) senão, buscar `"<artist> <title>"` (top 3, detalhes em paralelo), aplicar os filtros
   eliminatórios de E1 e pontuar com a **tolerância de clipe** (abaixo). Melhor ≥ 0.80 ⇒ trocar
   a fonte do download pelo id oficial (em jobs de fila; no preview é sugestão pré-selecionada
   que o usuário pode desmarcar). Guardar o ISRC encontrado (vai para `library.isrc`).
4. **Enriquecimento** (se `fetchMetadata` e não `offlineMode`): consultar provedores na ordem
   Deezer → iTunes → MusicBrainz (e Spotify/Discogs se configurados, F13) em paralelo
   respeitando limites de taxa; cada um devolve até 5 candidatos; pontuar todos.
5. **Decisão**: se a base é oficial ⇒ usar candidato só para **completar** campos vazios
   (gênero, faixa, total, disco, capa) e só se score ≥ `confidenceAutoApply`.
   Se não é oficial: melhor score ≥ `confidenceAutoApply` ⇒ aplicar;
   `confidenceReview` ≤ score < auto ⇒ manter base, `needs_review = 1`, guardar top 5 em
   `review_candidates_json`; abaixo ⇒ manter base, sem revisão.
6. Cache: chave `sha256(provider|normalized artist|normalized title|round(duration))`, validade 30 dias.

### Provedores (sem chave)
| Provedor | Endpoint | Limite | Campos |
|----------|----------|--------|--------|
| Deezer | `GET https://api.deezer.com/search?q=<artista> <título>&limit=5` (busca **simples** — a forma avançada combinada `artist:"A" track:"T"` retorna 0 resultados, verificado em 2026-10-01); detalhes `GET /track/{id}` (`isrc`, track_position, disk_number, release_date) e `GET /album/{id}` (genres); por ISRC: `GET /track/isrc:{ISRC}` | 5 req/s | title, artist.name, album.title, duration, album.cover_xl (1000 px), isrc |
| iTunes | `GET https://itunes.apple.com/search?term=<A T>&entity=song&limit=5` | 3 req/s | trackName, artistName, collectionName, trackTimeMillis, releaseDate, primaryGenreName, trackNumber, trackCount, discNumber, artworkUrl100 (trocar `100x100bb` por `1200x1200bb`) |
| MusicBrainz | `GET https://musicbrainz.org/ws/2/recording?query=recording:"T" AND artist:"A"&fmt=json&limit=5` | **1 req/s** (obrigatório) | title, artist-credit, length (ms), releases[] (title, date, id, media/track position); capa via Cover Art Archive `https://coverartarchive.org/release/{mbid}/front-1200` |
| LRCLIB (F09) | `GET https://lrclib.net/api/get?artist_name=&track_name=&album_name=&duration=`; fallback `GET /api/search?track_name=&artist_name=` | 5 req/s | plainLyrics, syncedLyrics, duration |

Todas as requisições com `User-Agent: Reverb/<versão> (+https://github.com/<owner>/<repo>)`
(MusicBrainz exige UA identificável) e timeout 15 s. Base URLs injetáveis (struct `Endpoints`)
para testes com `wiremock`.

### §11.2 Pontuação
```
norm(s): NFKD → remover diacríticos → minúsculas → remover trechos entre ()/[] que contenham
  official|video|vídeo|clipe|audio|áudio|lyric|lyrics|letra|legendado|visualizer|hd|hq|4k|
  remaster|remastered|mv|oficial|explicit
  → remover "feat. …", "ft. …", "featuring …" (só para comparar) → "&" ⇒ "and"
  → remover pontuação → colapsar espaços → trim
sim(a,b) = jaro_winkler(norm a, norm b); se um contém o outro inteiro e ambos ≥ 3 chars ⇒ max(sim, 0.95)
title_sim  = sim(títulos)
artist_sim = máximo de sim entre todos os pares (artistas da base × artistas do candidato)
dur_score  = tolerância normal: Δ ≤ 2 s ⇒ 1; Δ ≥ 10 s ⇒ 0; linear entre
             tolerância de clipe (fonte é clipe, não oficial): Δ ≤ 5 ⇒ 1; Δ ≥ 45 ⇒ 0; linear
score = 0.45·title_sim + 0.35·artist_sim + 0.20·dur_score
se qualquer lado não tem duração: score = 0.55·title_sim + 0.45·artist_sim, e score = min(score, 0.84)
penalidade de versão: se exatamente um lado contém (live|ao vivo|remix|acoustic|acústico|
  instrumental|karaoke|cover|sped up|slowed|nightcore|8d) ⇒ score × 0.6
```

### §11.3 Análise de título (`parse_title`)
Convenção do YouTube é **"Artista - Título"** (o parser antigo em
`referencias/parseTrackTitle.antigo.ts` invertia — não copie esse erro).
1. Separadores: ` - `, ` – `, ` — `, ` | ` (o primeiro encontrado). Esquerda = artista, direita = título.
2. Sem separador ⇒ título = tudo; artista = canal sem ` - Topic`/`VEVO`/`Official`.
3. Remover do título os trechos de §11.2 (official video etc.), mas **manter** marcadores de versão (live, remix…).
4. Extrair `feat.` do título para o campo artista como `A feat. B`.
5. Aspas envolvendo o título (`"Title"`, `“Title”`) são removidas.

## §12. Pós-processamento (F09)

Ordem (depois de `identify`): `artwork → lyrics → loudness → tagging → organize`.
- **Capa**: candidatos na ordem: capa do candidato aplicado (quando a fonte final for
  Deezer/iTunes/MusicBrainz, ou quando um candidato completou uma faixa oficial com score ≥
  `confidenceAutoApply`) → capas dos **demais candidatos** já obtidos no `identify` com score ≥
  `confidenceAutoApply`, na ordem Deezer, iTunes, CAA (não fazer novas buscas) → miniatura do
  YouTube (sempre a última opção;
  em faixas oficiais do YouTube Music a miniatura é a capa do álbum dentro de um quadro 16:9). Baixar, decodificar com `image` (falhou ⇒ próximo). Se razão de aspecto > 1.05
  ⇒ recorte central quadrado. Redimensionar para no máx. 1200×1200 (Lanczos3). JPEG q90.
  Embutir como capa frontal. Se `writeFolderCover` ⇒ `cover.jpg` na pasta do álbum (não sobrescrever).
- **Letras**: LRCLIB `get` (duração ±2 s) ⇒ senão `search` e escolher o de menor Δ duração
  (≤ 3 s). Sincronizada ⇒ `.lrc` ao lado (se `writeLrcFile`) e embutir o texto LRC; senão embutir a simples.
- **Loudness** (`normalizeVolume`): `ffmpeg -hide_banner -nostats -i <f> -af ebur128=peak=true -f null -`
  ⇒ ler do resumo `I: <x> LUFS` e `Peak: <y> dBFS`. `REPLAYGAIN_TRACK_GAIN = -18 - I` (formato
  `"-3.80 dB"`), `REPLAYGAIN_TRACK_PEAK = 10^(y/20)` (6 casas). Para `.opus` também
  `R128_TRACK_GAIN = round((-23 - I) * 256)` (inteiro). Não altera o áudio.
- **Tags** (lofty, por formato: ID3v2.4 p/ mp3, MP4 ilst p/ m4a, VorbisComments p/ opus/flac):
  title, artist, album, album artist, year, track/total, disc, genre, lyrics, comentário
  `"Reverb · <source_url>"`, capa, ReplayGain. Depois de gravar, **reler** e validar.
- **Organizar**: renderizar `fileTemplate` (§13) ⇒ caminho final; mover atomicamente
  (`rename`; se falhar por volume diferente ⇒ copiar + `sync_all` + remover origem); inserir em
  `library` (FTS atualizado por trigger); `job.output_path`, `job.library_id`.

## §13. Modelo de nomes e sanitização

Variáveis: `{artist}`, `{albumartist}` (= album_artist ou artist), `{album}`, `{title}`,
`{track}`, `{track:02}`, `{disc}`, `{year}`, `{genre}`, `{channel}`, `{source_id}`,
`{playlist}`, `{playlist_index}`, `{playlist_index:03}`.
Regras:
1. Variável vazia: `{album}` vazio ⇒ `Singles`; `{albumartist}`/`{artist}` vazio ⇒ `Artista desconhecido`
   (texto do idioma atual); demais vazias ⇒ remover a variável **e** o separador ` - ` adjacente
   (ex.: `{track:02} - {title}` sem faixa ⇒ `{title}`).
2. Conteúdo `other` ignora o modelo: `Outros/{channel}/{title}` (pasta "Outros"/"Other" pelo idioma).
3. `autoOrganize = false` ⇒ `{artist} - {title}` direto em `outputDir`.
4. Sanitização **sempre com regras do Windows** (vale também no Linux, para portabilidade):
   remover `< > : " / \ | ? *` e controles (substituir por `_` exceto `:` ⇒ ` -`), remover pontos
   e espaços finais de cada componente, prefixar `_` em nomes reservados (`CON PRN AUX NUL COM1-9 LPT1-9`,
   com ou sem extensão), limitar cada componente a 120 caracteres (cortar em limite de
   caractere Unicode, nunca no meio de um), caminho total ≤ 240 caracteres (encurtar o título).
5. Colisão: mesmo `source_id`+perfil já na biblioteca ⇒ é duplicata (tratada antes de enfileirar);
   arquivo existente diferente ⇒ sufixo ` (2)`, ` (3)`…

## §14. Classificação de URLs (`url_kind`, função pura no core, espelhada em TS só para UI)

| Entrada | Tipo |
|---------|------|
| `youtube.com/watch?…v=ID` (qualquer ordem de parâmetros), `youtu.be/ID`, `youtube.com/shorts/ID`, `youtube.com/live/ID`, `m.youtube.com/…`, `music.youtube.com/watch?v=ID` | `video` (se também tiver `list=`, `video` com `playlist_hint`) |
| `youtube.com/playlist?list=…`, `music.youtube.com/playlist?list=…`, `music.youtube.com/browse/MPREb_…` | `collection` |
| `youtube.com/@handle`, `/@handle/videos`, `/channel/UC…`, `/c/…`, `/user/…` | `collection` (canal). **Normalizar** para a aba de vídeos: sem aba ⇒ acrescentar `/videos` (sem isso o yt-dlp devolve as abas do canal como sub-playlists) |
| `soundcloud.com/…`, `*.bandcamp.com/…`, `archive.org/details/…`, `jamendo.com/…` | `video`/`collection` do provedor (F14; antes disso ⇒ `unsupported`) |
| `deezer.com/<lang>/playlist/<id>`, `/album/<id>`, `/track/<id>`, `link.deezer.com/s/…` (seguir redirecionamento); `open.spotify.com/playlist/<id>`, `/album/<id>`, `/track/<id>` (com ou sem `intl-xx/`, ignorar `?si=`) | `import` (F15; antes ⇒ `unsupported`) |
| `deezer.com/<lang>/artist/<id>` | `artist` (F15: seguir artista) |
| texto sem esquema | `search` |
| qualquer outra URL | `unsupported` |

## §15. IPC: comandos e eventos

Comandos Rust (`snake_case`), wrappers TS em `src/lib/ipc/api.ts` (`camelCase`), tipos via ts-rs.
Erro de comando: `{ kind: string, message: string, i18nKey?: string }`.

| Grupo | Comandos (fase em que nasce) |
|-------|----------|
| App | `app_info` (F00), `app_restart` (F06), `diagnostics_run`, `logs_export`, `data_export`, `data_import`, `open_data_dir` (F12), `open_output_dir` (F07), `clipboard_read_text` (F07) |
| Config | `settings_get`, `settings_update`, `settings_reset` (F01), `template_preview` (F09) |
| Ferramentas | `tools_status`, `tools_install_missing`, `tools_check_updates`, `tools_update`, `tools_rollback` (F02) |
| Análise | `url_classify`, `analyze`, `search` (F07), `find_official_version`, `metadata_preview`, `metadata_search` (F08) |
| Fila | `enqueue`, `check_duplicates`, `jobs_list`, `job_cancel`, `job_retry`, `job_remove`, `job_move`, `jobs_clear_finished`, `queue_pause`, `queue_resume`, `queue_state`, `jobs_cancel_all` (F04 core / F07 UI) |
| Biblioteca | `library_reveal` (F07), `library_cover` (F09), `library_list`, `library_get`, `library_artists`, `library_albums`, `library_delete`, `library_open_file`, `library_import`, `library_rescan`, `library_clear` (F10) |
| Revisão/tags | `pick_folder` (F07), `review_list`, `review_apply`, `review_dismiss`, `tags_read`, `tags_write`, `pick_audio_file`, `pick_image_file`, `artwork_fetch` (F10) |
| Playlists | `syncs_list`, `sync_create`, `sync_update`, `sync_delete`, `sync_run`, `sync_items` (F11) |
| Atualizações | `updater_check`, `updater_install` (F06) |
| Integração | `bookmarklet_code`, `deeplink_test` (F12), `cookies_test` (F13) |
| Qualidade | `upgrade_scan`, `upgrade_enqueue`, `trim_audio`, `waveform` (F13), `verify_lossless` (F14) |
| Provedores | `provider_search` (F14) |
| Importação/coleção (F15) | `import_analyze` (URL Deezer/Spotify ⇒ lista de faixas + casamento), `import_enqueue`, `artists_search`, `artist_follow`, `artist_update`, `artist_unfollow`, `artists_followed`, `artist_releases`, `artists_check_now`, `missing_list`, `missing_download` |

Comandos novos que uma fase precisar e não estiverem aqui: adicionar a esta tabela (registrar
no PROGRESS.md). O script `check:ipc` (F05) garante que **Rust (`generate_handler!`), wrappers
TS (`COMMANDS` em `api.ts`) e backend mock** têm exatamente o mesmo conjunto de comandos.

Eventos: `job://updated` (Job), `job://removed` ({id}), `queue://state`, `tools://progress`,
`tools://changed`, `heal://state`, `settings://changed`, `library://changed` ({ids}),
`library://import-progress`, `import://progress` (F15), `artists://updated` (F15), `sync://updated`, `updater://available`, `updater://progress`,
`clipboard://url`, `deeplink://received`, `notice` ({level, i18nKey, params}).

**Capabilities** (`src-tauri/capabilities/default.json`), somente:
`core:default`, `core:window:allow-start-dragging`, `core:window:allow-minimize`,
`core:window:allow-toggle-maximize`, `core:window:allow-close`, `core:window:allow-hide`,
`core:window:allow-show`, `core:window:allow-set-focus`, `core:window:allow-is-maximized`.
Comandos próprios do app não precisam de permissão (são do próprio app). Um teste (F16) garante
que nenhuma permissão extra foi adicionada.

## §16. Gerenciador de ferramentas (F02)

| Ferramenta | Fonte (GitHub releases API `…/releases/latest`) | Asset Windows x64 | Asset Linux x64 | Checksum |
|------------|----------------------|-------------------|-----------------|----------|
| yt-dlp stable | `yt-dlp/yt-dlp` | `yt-dlp.exe` | `yt-dlp_linux` | `SHA2-256SUMS` (linhas `<hash>  <nome>`) |
| yt-dlp nightly | `yt-dlp/yt-dlp-nightly-builds` | `yt-dlp.exe` | `yt-dlp_linux` | `SHA2-256SUMS` |
| Deno | `denoland/deno` | `deno-x86_64-pc-windows-msvc.zip` | `deno-x86_64-unknown-linux-gnu.zip` | `<asset>.sha256sum` (pegar o 1º token de 64 hex) |
| FFmpeg+ffprobe | `yt-dlp/FFmpeg-Builds` (tag fixa `latest`, rolante) | `ffmpeg-master-latest-win64-gpl.zip` | `ffmpeg-master-latest-linux64-gpl.tar.xz` | `checksums.sha256` |
| fpcalc (opcional) | `acoustid/chromaprint` | `chromaprint-fpcalc-<v>-windows-x86_64.zip` | `chromaprint-fpcalc-<v>-linux-x86_64.tar.gz` | sem checksum ⇒ registrar aviso |
| bgutil (PO token, estudo E4) | `Brainicism/bgutil-ytdlp-pot-provider` | plugin: asset `bgutil-ytdlp-pot-provider.zip` (usado sem extrair, via `--plugin-dirs`); servidor: `zipball_url` da mesma tag ⇒ pasta `server/` ⇒ `deno install --allow-scripts=npm:canvas --frozen` | idem | sem checksum ⇒ registrar aviso; teste de fumaça = subir servidor e `GET /ping` devolver a versão |

Seleção de asset por **regex** (não por nome fixo), usando a tabela acima como padrão.
Versões: yt-dlp `--version` (`2026.08.19` / nightly `2026.09.27.232945`) vs `tag_name`;
Deno `deno --version` (1ª linha `deno X.Y.Z`) vs tag `vX.Y.Z`; FFmpeg: tag rolante ⇒ comparar
`updated_at` do asset com o gravado no manifesto; fpcalc: tag `vX.Y.Z`.
Requisições à API do GitHub com header `User-Agent` (obrigatório) e `Accept: application/vnd.github+json`;
usar `GITHUB_TOKEN` do ambiente se existir (CI). Limite anônimo: 60/h ⇒ cache das respostas por 1 h.
Instalação: baixar para `tools/.staging/` com progresso ⇒ verificar hash ⇒ extrair (zip via crate;
`.tar.xz`/`.tar.gz` via comando `tar` do sistema no Linux) ⇒ `chmod 755` (Unix) ⇒ executar
`--version`/`-version` como teste de fumaça ⇒ mover para `tools/<nome>/<versão>/` ⇒ atualizar
`tools/manifest.json` (`current`, `previous`, `channel`, `installed_at`, `asset_updated_at`) ⇒
apagar versões além de current+previous.
Exclusão mútua: `RwLock` — runners de yt-dlp/ffmpeg seguram leitura; instalar/rollback pega
escrita (emite `tools://progress {phase:"waiting_jobs"}` enquanto espera).
Runtime JS (`jsRuntime`): `auto` = deno do sistema (PATH) se existir ⇒ senão node do sistema
≥ 20 ⇒ senão deno gerenciado (baixar). Os demais valores forçam a escolha.
Verificação automática: yt-dlp 1×/dia (e na autocura), Deno e FFmpeg 1×/semana, se `autoUpdateTools`.

## §17. Atualização do app (F06)

- `tauri-plugin-updater` usado pelo Rust. Endpoint
  `https://github.com/<owner>/<repo>/releases/latest/download/latest.json`; `pubkey` da chave
  gerada na F06. Instaladores: Windows **NSIS** (`installMode: "currentUser"`), Linux **AppImage**
  (único formato Linux com autoatualização; `.deb` é gerado também, sem autoatualização).
  Os alvos ficam nos arquivos de configuração por plataforma (mesclados automaticamente pelo
  Tauri): `tauri.windows.conf.json` ⇒ `{"bundle":{"targets":["nsis"]}}`;
  `tauri.linux.conf.json` ⇒ `{"bundle":{"targets":["appimage","deb"]}}`.
  `bundle.createUpdaterArtifacts: true`. Updater no Windows com `installMode: "passive"`.
  (Transparência/Mica **não** vão nesses arquivos: a janela é criada em código — ver o último
  item desta seção e a F05, tarefa 11.)
- Verifica na inicialização (se `autoCheckAppUpdates`) e a cada 6 h. Botão "Verificar
  atualizações" verifica app + ferramentas. "Atualizar agora" baixa com progresso, instala e reinicia.
- Release = tag `vX.Y.Z` ⇒ workflow `release.yml` (tauri-action, matriz windows-latest +
  ubuntu-22.04, `releaseDraft: false`, `includeUpdaterJson: true`). Versão única em
  `package.json`; `tauri.conf.json` usa `"version": "../package.json"`; `scripts/bump-version.mjs`
  sincroniza os `Cargo.toml`.
- Flags headless do executável (para testes automatizados, sem abrir janela):
  `--headless-selftest <arquivo.json>`, `--headless-update-check <arquivo.json>`,
  `--headless-update-install`. A janela principal é criada em `setup()` (não em
  `tauri.conf.json`) para que os modos headless não abram janela.

## §18. Logs e privacidade

- `tracing` com arquivo diário em `logs/` (manter 14 dias) + stderr em dev.
- Filtro de redação: valores de chaves de API, cookies, tokens e query strings `key=`, `token=`,
  `client_secret=`, `sig=`, `signature=` são trocados por `***`. Teste obrigatório (F01).
- Sem telemetria. Nada sai da máquina além das requisições necessárias às funções.
- `logs_export` gera zip com logs (já redigidos) + `diagnostics.json`.

## §19. Testes — política geral

| Camada | Ferramenta | Onde | Rede? |
|--------|-----------|------|-------|
| Unidade/integração core | `cargo test` | `crates/*/src` e `crates/*/tests` | não |
| Rede real | `cargo test -- --ignored` com `REVERB_NET_TESTS=1` | `crates/reverb-core/tests/net_*.rs` | sim |
| UI unidade | Vitest + Testing Library + `mockIPC` | `src/**/*.test.tsx` | não |
| UI E2E | Playwright + backend mock | `tests/e2e` | não |
| App real | WebdriverIO + tauri-driver | `tests/e2e-app` | às vezes |
| CI | GitHub Actions (Windows + Ubuntu) | `.github/workflows/ci.yml` | **não** para YouTube (IPs de datacenter costumam ser bloqueados pelo YouTube; testes de rede só locais). O CI roda `npm run test:prepare` (baixa ferramentas do GitHub, isso funciona) antes de `npm run verify` |

- Áudio de teste: **gerado em tempo de execução** com ffmpeg (`-f lavfi -i sine=…` / `anoisesrc`), nunca commitado.
- Respostas HTTP de provedores: gravadas uma vez (na fase que cria o provedor) em
  `tests/fixtures/http/<provedor>/*.json` e servidas por `wiremock`.
- JSON do yt-dlp: gravados uma vez em `tests/fixtures/ytdlp/` (removendo URLs assinadas de formatos).
- Testes que precisam de ffmpeg/yt-dlp reais localizam-nos em `.test-tools/` (instalados por
  `npm run test:prepare`) ou via `REVERB_TEST_TOOLS_DIR`. Se não encontrados ⇒ **falham** com
  mensagem clara "rode npm run test:prepare" (não pulam).

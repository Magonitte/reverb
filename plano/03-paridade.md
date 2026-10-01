# 03 — Paridade de funcionalidades (antigo → novo)

Regra: **nenhuma funcionalidade do app antigo pode se perder**, exceto as listadas em
"Remoções intencionais" (cada uma com o substituto). Na F16, cada linha abaixo deve estar
marcada `[x]` com o nome do teste que a cobre.

## Comandos do backend antigo (`src-tauri/src/commands.rs` + `sidecar.rs`)

| Antigo | Novo | Fase | Coberto por teste |
|--------|------|------|-------------------|
| `analyze_url` | `analyze` (+ `url_classify`) | F03/F07 | [ ] |
| `download_single` | `enqueue` (1 item) | F04/F07 | [ ] |
| `download_playlist` | `enqueue` (vários, `kind=playlist_item`) | F04/F07 | [ ] |
| `cancel_download` | `job_cancel` / `jobs_cancel_all` | F04 | [ ] |
| `get_queue` / `restore_queue` | `jobs_list` + restauração automática na inicialização | F04 | [ ] |
| `pause_queue` / `resume_queue` | `queue_pause` / `queue_resume` | F04 | [ ] |
| `check_duplicate` | `check_duplicates` (+ diálogo) | F04/F07 | [ ] |
| `get_history` | `library_list` (+ aba Concluídos da Atividade) | F09/F10 | [ ] |
| `delete_history_entry` | `library_delete` (sem/com arquivo) | F10 | [ ] |
| `clear_history` | `library_clear` (+ `jobs_clear_finished`) | F10 | [ ] |
| `read_tags` / `edit_tags` | `tags_read` / `tags_write` | F10 | [ ] |
| `search_metadata` | `metadata_search` | F08/F10 | [ ] |
| `select_audio_file` / `select_folder` | `pick_audio_file` / `pick_folder` | F07/F10 | [ ] |
| `open_download_folder` / `open_last_download_folder` | `library_reveal` + ação rápida "Abrir pasta" + item da bandeja | F07/F12 | [ ] |
| `check_ffmpeg` / `download_ffmpeg` | `tools_status` / `tools_install_missing` | F02 | [ ] |
| `ping_engine` | `diagnostics_run` (não há mais engine) | F12 | [ ] |
| `save_api_config` / `get_api_config_status` / `clear_api_config` | `settings_update` (chaves) + "Testar" | F13 | [ ] |
| `sync_app_preferences` | `settings_get` / `settings_update` (backend é a fonte da verdade) | F01 | [ ] |
| `export_telemetry` | `logs_export` (sem telemetria) | F12 | [ ] |

## Configurações antigas (`settingsStore.ts`)

| Antigo | Novo | Fase | Teste |
|--------|------|------|-------|
| outputDir | outputDir | F01 | [ ] |
| defaultFormat + defaultQuality | defaultProfile (perfis §7) | F03 | [ ] |
| parallelism (1–4) | parallelism | F04 | [ ] |
| speedLimitMbps | speedLimitMbps | F04 | [ ] |
| queueLimit | queueLimit | F04 | [ ] |
| fetchMetadata / fetchLyrics / fetchArtwork | idem | F08/F09 | [ ] |
| extractTitleFromVideo | idem (parser corrigido "Artista - Título") | F08 | [ ] |
| removeSponsorBlocks | sponsorblockRemove + categorias | F03 | [ ] |
| autoOrganize | autoOrganize + fileTemplate | F09 | [ ] |
| offlineMode | offlineMode | F08 | [ ] |
| normalizeVolume (EBU R128) | normalizeVolume (ReplayGain, não destrutivo) | F09 | [ ] |
| launchWithWindows | launchAtStartup (+ startMinimized) | F12 | [ ] |
| minimizeToTray | minimizeToTray + closeToTray | F12 | [ ] |
| completionNotifications | idem | F12 | [ ] |
| theme (dark/light/system) | idem (system = segue o SO) | F05 | [ ] |
| language (pt-BR/en) | idem | F05 | [ ] |
| ffmpegMissing / ffmpegPath | gerenciador de ferramentas (FFmpeg obrigatório) | F02 | [ ] |

## Telas e recursos de UI antigos

| Antigo | Novo | Fase | Teste |
|--------|------|------|-------|
| Home: campo de URL + Analisar | Barra de comando no Início | F07 | [ ] |
| Home: ações rápidas (playlist, editar metadados, ver fila) | Ações rápidas do Início | F07 | [ ] |
| Home: downloads recentes | Recentes (grade de capas) | F07/F10 | [ ] |
| Preview: miniatura, título, canal, duração | Preview | F07 | [ ] |
| Preview: formatos (MP3 320, AAC 256, FLAC, Opus 160) | Perfis (original, mp3_v0, mp3_320, aac_256, opus_96, flac) | F03/F07 | [ ] |
| Preview: opções (metadados, letra, capa, SponsorBlock, organizar) | idem | F07 | [ ] |
| Preview: escolher pasta de destino | idem | F07 | [ ] |
| Preview: "Baixar áudio" × "Adicionar à fila" | "Baixar agora" × "Adicionar à fila" | F07 | [ ] |
| MetadataPreviewModal / MetadataEditModal | "Pré-visualizar metadados" + edição antes de baixar | F08 | [ ] |
| PlaylistModal: seleção de faixas + limite | Coleção | F07 | [ ] |
| Fila: contadores, pausar, retomar, cancelar todos, remover, tentar de novo, "Tentativa x/y", falha permanente, aviso de limite | Atividade | F07 | [ ] |
| Fila: reordenar (drag handle) | Atividade (dnd-kit) | F07 | [ ] |
| Histórico: busca, filtros formato/data, colunas, abrir pasta, editar tags, baixar novamente, limpar tudo | Biblioteca | F10 | [ ] |
| Diálogo de download duplicado | idem | F07 | [ ] |
| Editor de metadados / TagEditor | Editor de tags | F10 | [ ] |
| Configurações: abas Geral/Áudio/Metadados/Avançado + APIs | Abas Geral/Downloads/Metadados/Integração/Atualizações/Avançado | F05+ | [ ] |
| Onboarding (boas-vindas, ambiente, FFmpeg, APIs, pronto) | Onboarding novo | F12 | [ ] |
| UpdateChecker (atualizador) | Atualizações (app + ferramentas) + botão | F06 | [ ] |
| Bandeja: título, nº ativos, Abrir, Sair | Bandeja (+ pausar/retomar, baixar link copiado) | F12 | [ ] |
| Notificações nativas de conclusão | idem (+ resumo de lote) | F12 | [ ] |
| Estado da janela lembrado (window-state) | idem | F12 | [ ] |
| Modo portátil (`portable.txt`) | idem | F01 | [ ] |
| Atalhos Ctrl+V/D/Q/H/,, Espaço, Esc, Enter | idem + Ctrl+K | F05 | [ ] |
| i18n pt-BR / en | idem | F05 | [ ] |
| Titlebar customizada + Mica | idem (Mica só Windows) | F05/F06 | [ ] |
| Metadados: cache, Spotify, MusicBrainz, Discogs, fallback yt-dlp | cache, YouTube Music, Deezer, iTunes, MusicBrainz, (Spotify, Discogs opcionais), base do vídeo | F08/F13 | [ ] |
| Capas (Cover Art Archive) | CAA + Deezer + iTunes + miniatura recortada | F09 | [ ] |
| Letras (Genius) | LRCLIB (sincronizadas) | F09 | [ ] |

## Remoções intencionais (com substituto)

| Removido | Motivo | Substituto |
|----------|--------|-----------|
| Sidecar Python / `ping_engine` | complexidade, quebra por yt-dlp congelado | core Rust + yt-dlp gerenciado + diagnóstico |
| Genius (letras) | API oficial não fornece letras (exige raspagem frágil) | LRCLIB |
| "Modo rápido" sem FFmpeg | FFmpeg é necessário para remux/ReplayGain/capítulos | instalação automática do FFmpeg |
| Tema "Automático (horário)" | substituído por seguir o SO | `system` |
| Telemetria | uso pessoal, privacidade | `logs_export` |
| Idioma "es" (só existia no tipo, sem tradução) | nunca funcionou | — |

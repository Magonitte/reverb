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

-- FTS5 "external content": o comando 'delete' é obrigatório para manter o índice coerente.
CREATE TRIGGER library_fts_ai AFTER INSERT ON library BEGIN
  INSERT INTO library_fts(rowid, title, artist, album)
  VALUES (new.id, new.title, new.artist, new.album);
END;
CREATE TRIGGER library_fts_ad AFTER DELETE ON library BEGIN
  INSERT INTO library_fts(library_fts, rowid, title, artist, album)
  VALUES ('delete', old.id, old.title, old.artist, old.album);
END;
CREATE TRIGGER library_fts_au AFTER UPDATE ON library BEGIN
  INSERT INTO library_fts(library_fts, rowid, title, artist, album)
  VALUES ('delete', old.id, old.title, old.artist, old.album);
  INSERT INTO library_fts(rowid, title, artist, album)
  VALUES (new.id, new.title, new.artist, new.album);
END;

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

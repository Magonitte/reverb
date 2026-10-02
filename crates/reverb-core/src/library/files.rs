//! Importação e reexame: paths can be files or folders; symbolic links are not followed.
use crate::tagging::TrackTags;
use crate::{CoreError, CoreResult, Db, EventSink};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportFailure {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportReport {
    pub imported: u32,
    pub skipped: u32,
    pub failures: Vec<ImportFailure>,
}

pub fn canonical(path: &Path) -> CoreResult<PathBuf> {
    let path = std::fs::canonicalize(path)?;
    #[cfg(windows)]
    let path = PathBuf::from(
        path.to_string_lossy()
            .strip_prefix(r"\\?\")
            .unwrap_or(&path.to_string_lossy()),
    );
    Ok(path)
}

/// Upgrade existing F09 paths without deleting records or replacing conflicting entries.
pub fn normalize_paths(conn: &rusqlite::Connection) -> CoreResult<usize> {
    let paths = {
        let mut stmt = conn.prepare("SELECT id,file_path FROM library")?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };
    let mut changed = 0;
    for (id, path) in paths {
        if let Ok(normalized) = canonical(Path::new(&path)) {
            let normalized = normalized.to_string_lossy();
            if normalized != path {
                changed += conn.execute(
                    "UPDATE OR IGNORE library SET file_path=? WHERE id=?",
                    params![normalized, id],
                )?;
            }
        }
    }
    Ok(changed)
}

pub(super) fn supported(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        ["mp3", "m4a", "opus", "ogg", "flac", "wav"]
            .contains(&ext.to_string_lossy().to_ascii_lowercase().as_str())
    })
}

fn collect(path: &Path, files: &mut BTreeSet<PathBuf>, failures: &mut Vec<ImportFailure>) {
    let result = (|| -> CoreResult<()> {
        let metadata = std::fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() {
            return Ok(());
        }
        if path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(".reverb-"))
        {
            return Ok(());
        }
        if metadata.is_dir() {
            for entry in std::fs::read_dir(path)? {
                collect(&entry?.path(), files, failures);
            }
        } else if metadata.is_file() && supported(path) {
            files.insert(canonical(path)?);
        }
        Ok(())
    })();
    if let Err(error) = result {
        failures.push(ImportFailure {
            path: path.to_string_lossy().into_owned(),
            message: error.to_string(),
        });
    }
}

/// Idempotent by canonical path, including overlapping selected folders and files.
pub async fn import(
    db: &Db,
    paths: Vec<PathBuf>,
    ffprobe: &Path,
    sink: Arc<dyn EventSink>,
    origin: &'static str,
) -> CoreResult<ImportReport> {
    if origin == "import" {
        db.call(|conn| normalize_paths(conn)).await?;
    }
    let (files, mut report) = tokio::task::spawn_blocking(move || {
        let mut files = BTreeSet::new();
        let mut report = ImportReport::default();
        for path in paths {
            collect(&path, &mut files, &mut report.failures);
        }
        (files, report)
    })
    .await
    .map_err(|e| CoreError::Internal(e.to_string()))?;
    let total = files.len();
    for (index, path) in files.into_iter().enumerate() {
        let text = path.to_string_lossy().into_owned();
        let existing = text.clone();
        let exists = db
            .call(move |conn| {
                let exists = conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM library WHERE file_path=?)",
                    [&existing],
                    |r| r.get::<_, bool>(0),
                )?;
                if exists {
                    conn.execute("UPDATE library SET missing=0,updated_at=unixepoch() WHERE file_path=? AND missing=1",[existing])?;
                }
                Ok(exists)
            })
            .await?;
        if exists || crate::organize::is_moving(&path) {
            report.skipped += 1;
        } else {
            let tags_path = path.clone();
            let tags = tokio::task::spawn_blocking(move || crate::tagging::read_tags(&tags_path))
                .await
                .map_err(|e| CoreError::Internal(e.to_string()))?;
            let result = match tags {
                Err(e) => Err(e),
                Ok(mut tags) => {
                    if tags.title.trim().is_empty() {
                        tags.title = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned();
                    }
                    match crate::transcode::probe(ffprobe, &path).await {
                        Err(e) => Err(CoreError::coded(e.kind.as_str(), e.message)),
                        Ok(probe) => {
                            let file = text.clone();
                            db.call(move |conn| {
                                if crate::organize::is_moving(Path::new(&file)) { return Ok(false); }
                                let now=crate::queue::repo::now();
                                let lyrics=tags.lyrics.as_deref().filter(|v|!v.trim().is_empty());
                                let synced=lyrics.is_some_and(|v|v.lines().any(|line|line.trim_start().starts_with('[') && line.contains(':')));
                                let inserted=conn.execute("INSERT INTO library(file_path,title,artist,album,album_artist,year,genre,track_no,track_total,disc_no,isrc,duration_s,codec,bitrate_kbps,has_lyrics,has_synced_lyrics,origin,added_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(file_path) DO NOTHING",params![file,tags.title,tags.artist,tags.album,tags.album_artist,tags.year,tags.genre,tags.track_no,tags.track_total,tags.disc_no,tags.isrc,probe.duration_s,probe.codec,probe.bitrate_kbps,lyrics.is_some(),synced,origin,now,now])?;
                                Ok(inserted>0)
                            }).await
                        }
                    }
                }
            };
            match result {
                Ok(true) => report.imported += 1,
                Ok(false) => report.skipped += 1,
                Err(error) => report.failures.push(ImportFailure {
                    path: text,
                    message: error.to_string(),
                }),
            }
        }
        sink.emit("library://import-progress",serde_json::json!({"processed":index+1,"total":total,"imported":report.imported,"skipped":report.skipped,"failed":report.failures.len()}));
    }
    sink.emit("library://changed", serde_json::json!({}));
    Ok(report)
}

pub async fn rescan(
    db: &Db,
    root: PathBuf,
    ffprobe: &Path,
    sink: Arc<dyn EventSink>,
) -> CoreResult<ImportReport> {
    db.call(|conn| normalize_paths(conn)).await?;
    let paths = db
        .call(|conn| {
            let mut stmt = conn.prepare("SELECT id,file_path FROM library")?;
            let values = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(values)
        })
        .await?;
    let states = tokio::task::spawn_blocking(move || {
        paths
            .into_iter()
            .map(|(id, path)| (id, !Path::new(&path).is_file()))
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|e| CoreError::Internal(e.to_string()))?;
    db.call(move |conn| {
        let tx = conn.transaction()?;
        for (id, missing) in states {
            tx.execute(
                "UPDATE library SET missing=?,updated_at=unixepoch() WHERE id=? AND missing<>?",
                params![missing, id, missing],
            )?;
        }
        tx.commit()?;
        Ok(())
    })
    .await?;
    import(db, vec![root], ffprobe, sink, "scan").await
}

/// Updates only existing records. An external file never becomes an implicit import.
pub fn update_tags(conn: &rusqlite::Connection, path: &str, tags: &TrackTags) -> CoreResult<()> {
    let lyrics = tags.lyrics.as_deref().filter(|s| !s.trim().is_empty());
    let synced = lyrics.is_some_and(|v| {
        v.lines()
            .any(|line| line.trim_start().starts_with('[') && line.contains(':'))
    });
    conn.execute("UPDATE library SET title=?,artist=?,album=?,album_artist=?,year=?,genre=?,track_no=?,track_total=?,disc_no=?,isrc=?,has_lyrics=?,has_synced_lyrics=?,missing=0,updated_at=unixepoch() WHERE file_path=?",params![tags.title,tags.artist,tags.album,tags.album_artist,tags.year,tags.genre,tags.track_no,tags.track_total,tags.disc_no,tags.isrc,lyrics.is_some(),synced,path])?;
    Ok(())
}

/// Editing an external file does not import it. A failed move restores the original tags.
pub fn write(
    conn: &mut rusqlite::Connection,
    path: &Path,
    tags: &TrackTags,
    reorganize: bool,
    settings: &crate::Settings,
) -> CoreResult<PathBuf> {
    let source = canonical(path)?;
    let text = source.to_string_lossy().into_owned();
    let id = conn
        .query_row(
            "SELECT id FROM library WHERE file_path IN (?,?)",
            params![text, path.to_string_lossy()],
            |r| r.get::<_, i64>(0),
        )
        .optional()?;
    let item = id.map(|id| super::get(conn, id)).transpose()?.flatten();
    let mut target = source.clone();
    if let Some(item) = &item {
        if reorganize && settings.auto_organize {
            let context = crate::organize::template::TemplateContext {
                output_dir: crate::paths::resolve_output_dir(settings),
                extension: source
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                language: settings.language,
                content_type: item.content_type,
                auto_organize: true,
                channel: item.artist.clone(),
                source_id: item.source_id.clone(),
                playlist: None,
                playlist_index: None,
            };
            target = crate::organize::template::render_destination(
                &settings.file_template,
                tags,
                &context,
            )?;
            if canonical(&target).is_ok_and(|path| path == source) {
                target = source.clone();
            } else if target != source {
                let has_lrc = source.with_extension("lrc").is_file();
                target = crate::organize::sanitize::unique_path_for(&target, |path| {
                    path.exists() || (has_lrc && path.with_extension("lrc").exists())
                });
            }
        }
    }
    let original = crate::tagging::read_tags(&source)?;
    let mut moving = crate::organize::MovingPaths::default();
    moving.reserve(&source);
    moving.reserve(&target);
    crate::tagging::write_tags(&source, tags)?;
    let mut published = source.clone();
    let mut moved_lrc: Option<PathBuf> = None;
    let result = (|| -> CoreResult<()> {
        if target != source {
            if target.with_extension("lrc").exists() {
                return Err(CoreError::coded("disk", "Destination lyrics already exist"));
            }
            published = crate::organize::move_into_library(&source, &target)?;
            moving.reserve(&published);
            if source.with_extension("lrc").is_file() {
                moved_lrc = Some(crate::organize::move_into_library(
                    &source.with_extension("lrc"),
                    &published.with_extension("lrc"),
                )?);
                if moved_lrc.as_deref() != Some(published.with_extension("lrc").as_path()) {
                    return Err(CoreError::coded(
                        "disk",
                        "Destination lyrics changed during publication",
                    ));
                }
            }
        }
        published = canonical(&published)?;
        let tx = conn.transaction()?;
        if let Some(id) = id {
            tx.execute(
                "UPDATE library SET file_path=? WHERE id=?",
                params![published.to_string_lossy(), id],
            )?;
        }
        update_tags(&tx, &published.to_string_lossy(), tags)?;
        if original.cover != tags.cover {
            tx.execute(
                "UPDATE library SET cover_source=? WHERE file_path=?",
                params![
                    tags.cover.as_ref().map(|_| "user"),
                    published.to_string_lossy()
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    })();
    if let Err(error) = result {
        if let Some(lrc) = moved_lrc {
            crate::organize::move_into_library(&lrc, &source.with_extension("lrc"))?;
        }
        if published != source {
            crate::organize::move_into_library(&published, &source)?;
        }
        crate::tagging::write_tags(&source, &original)?;
        return Err(error);
    }
    Ok(published)
}

pub fn apply_review(
    conn: &mut rusqlite::Connection,
    id: i64,
    tags: &TrackTags,
    settings: &crate::Settings,
    source: &str,
) -> CoreResult<super::LibraryItem> {
    let item = super::get(conn, id)?
        .ok_or_else(|| CoreError::coded("not_found", "Library item missing"))?;
    write(conn, Path::new(&item.file_path), tags, true, settings)?;
    conn.execute("UPDATE library SET needs_review=0,review_candidates_json=NULL,metadata_source=?,confidence=1,updated_at=unixepoch() WHERE id=?", params![source,id])?;
    super::get(conn, id)?.ok_or_else(|| CoreError::coded("not_found", "Library item missing"))
}

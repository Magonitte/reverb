use super::Sync;
use crate::{CoreError, CoreResult, Settings};
use rusqlite::{params, Connection, OptionalExtension};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn output_dir(sync: &Sync, settings: &Settings) -> PathBuf {
    sync.output_dir
        .as_deref()
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::paths::resolve_output_dir(settings))
}

fn relative(file: &Path, base: &Path) -> CoreResult<String> {
    let file: Vec<_> = file.components().collect();
    let base: Vec<_> = base.components().collect();
    let common = file.iter().zip(&base).take_while(|(a, b)| a == b).count();
    if common == 0 {
        return Err(CoreError::coded(
            "disk",
            "Playlist and audio must share a filesystem root",
        ));
    }
    let mut result = PathBuf::new();
    for _ in common..base.len() {
        result.push("..");
    }
    for part in &file[common..] {
        result.push(part.as_os_str());
    }
    Ok(result.to_string_lossy().replace('\\', "/"))
}

fn line(text: &str) -> String {
    text.replace(['\r', '\n'], " ")
}

pub fn write_m3u(conn: &Connection, sync: &Sync, settings: &Settings) -> CoreResult<PathBuf> {
    let dir = output_dir(sync, settings).join("Playlists");
    std::fs::create_dir_all(&dir)?;
    let dir = crate::library::files::canonical(&dir)?;
    let path = crate::organize::sanitize_path(&dir, &[&sync.title], "m3u8");
    crate::organize::template::check_path(&path)?;
    let mut content = String::from("#EXTM3U\n");
    for item in super::repo::items(conn, &sync.id)? {
        if item.state != "present" || item.missing {
            continue;
        }
        let Some(id) = item.library_id else {
            continue;
        };
        let Some(track) = crate::library::get(conn, id)? else {
            continue;
        };
        let file = Path::new(&track.file_path);
        if !file.is_file() {
            continue;
        }
        let file = crate::library::files::canonical(file)?;
        let duration = track.duration_s.map_or(-1, |value| value.round() as i64);
        let title = match track.artist {
            Some(artist) if !artist.is_empty() => {
                format!("{} - {}", line(&artist), line(&track.title))
            }
            _ => line(&track.title),
        };
        content.push_str(&format!(
            "#EXTINF:{duration},{title}\n{}\n",
            relative(&file, &dir)?
        ));
    }
    let mut temp = tempfile::NamedTempFile::new_in(&dir)?;
    temp.write_all(content.as_bytes())?;
    temp.as_file().sync_all()?;
    temp.persist(&path)
        .map_err(|error| CoreError::coded("disk", error.to_string()))?;
    Ok(path)
}

/// Rename existing bytes; never runs the download pipeline or rewrites metadata.
pub fn rename_item(
    conn: &mut Connection,
    sync: &Sync,
    library_id: i64,
    position: u32,
    settings: &Settings,
) -> CoreResult<()> {
    let Some(item) = crate::library::get(conn, library_id)? else {
        return Ok(());
    };
    if item.missing || !Path::new(&item.file_path).is_file() {
        return Ok(());
    }
    let source = crate::library::files::canonical(Path::new(&item.file_path))?;
    let tags = crate::tagging::read_tags(&source)?;
    let context = crate::organize::template::TemplateContext {
        output_dir: output_dir(sync, settings),
        extension: source
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        language: settings.language,
        content_type: item.content_type,
        auto_organize: settings.auto_organize,
        channel: item.artist,
        source_id: item.source_id,
        playlist: Some(sync.title.clone()),
        playlist_index: Some(position),
    };
    let target =
        crate::organize::template::render_destination(&settings.file_template, &tags, &context)?;
    if target == source
        || crate::library::files::canonical(&target).is_ok_and(|path| path == source)
    {
        return Ok(());
    }
    let mut guard = crate::organize::MovingPaths::default();
    guard.reserve(&source);
    guard.reserve(&target);
    // Stage the old names so a database failure can restore them even for a collision.
    let backup = tempfile::Builder::new()
        .prefix(".reverb-sync-")
        .tempdir_in(
            source
                .parent()
                .ok_or_else(|| CoreError::coded("disk", "Audio has no parent"))?,
        )?;
    let staged = backup.path().join(source.file_name().unwrap_or_default());
    std::fs::rename(&source, &staged)?;
    let lyrics = source.with_extension("lrc");
    let staged_lyrics = staged.with_extension("lrc");
    if lyrics.is_file() {
        if let Err(error) = std::fs::rename(&lyrics, &staged_lyrics) {
            std::fs::rename(&staged, &source)?;
            return Err(error.into());
        }
    }
    let collision = target.is_file();
    let mut moved_audio = None;
    let mut moved_lyrics = None;
    let result = (|| -> CoreResult<()> {
        let published = if collision {
            crate::library::files::canonical(&target)?
        } else {
            let path = crate::organize::move_into_library(&staged, &target)?;
            moved_audio = Some(path.clone());
            crate::library::files::canonical(&path)?
        };
        if staged_lyrics.is_file() && !published.with_extension("lrc").exists() {
            moved_lyrics = Some(crate::organize::move_into_library(
                &staged_lyrics,
                &published.with_extension("lrc"),
            )?);
        }
        let tx = conn.transaction()?;
        let other: Option<i64> = tx
            .query_row(
                "SELECT id FROM library WHERE file_path=? AND id<>?",
                params![published.to_string_lossy(), library_id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(other) = other {
            tx.execute(
                "UPDATE sync_items SET library_id=? WHERE library_id=?",
                params![other, library_id],
            )?;
            tx.execute("DELETE FROM library WHERE id=?", [library_id])?;
        } else {
            tx.execute(
                "UPDATE library SET file_path=?,updated_at=unixepoch() WHERE id=?",
                params![published.to_string_lossy(), library_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    })();
    if let Err(error) = result {
        let restored = (|| -> CoreResult<()> {
            if let Some(path) = moved_lyrics {
                crate::organize::move_into_library(&path, &lyrics)?;
            } else if staged_lyrics.exists() {
                std::fs::rename(&staged_lyrics, &lyrics)?;
            }
            if let Some(path) = moved_audio {
                crate::organize::move_into_library(&path, &source)?;
            } else if staged.exists() {
                std::fs::rename(&staged, &source)?;
            }
            Ok(())
        })();
        if let Err(restore_error) = restored {
            let preserved = backup.keep();
            return Err(CoreError::coded(
                "disk",
                format!(
                    "{error}; {restore_error}; backup preserved at {}",
                    preserved.display()
                ),
            ));
        }
        return Err(error);
    }
    // Collision keeps the destination; the former source remains recoverable in Trash.
    let leftovers: Vec<_> = [staged, staged_lyrics]
        .into_iter()
        .filter(|path| path.exists())
        .collect();
    if !leftovers.is_empty() {
        if let Err(error) = trash::delete_all(leftovers) {
            let preserved = backup.keep();
            return Err(CoreError::coded(
                "disk",
                format!("{error}; source preserved at {}", preserved.display()),
            ));
        }
    }
    Ok(())
}

pub fn delete_tracks(conn: &mut Connection, ids: &[i64]) -> CoreResult<()> {
    let parents: std::collections::BTreeSet<_> = ids
        .iter()
        .filter_map(|id| crate::library::get(conn, *id).ok().flatten())
        .filter_map(|item| Path::new(&item.file_path).parent().map(PathBuf::from))
        .collect();
    crate::library::delete(conn, ids, true)?;
    for parent in parents {
        if !parent.is_dir() {
            continue;
        }
        let mut has_audio = false;
        for entry in std::fs::read_dir(&parent)? {
            let path = entry?.path();
            if path.is_file()
                && path.extension().is_some_and(|ext| {
                    ["mp3", "m4a", "opus", "ogg", "flac", "wav"]
                        .contains(&ext.to_string_lossy().to_ascii_lowercase().as_str())
                })
            {
                has_audio = true;
                break;
            }
        }
        let cover = parent.join("cover.jpg");
        if !has_audio && cover.is_file() {
            trash::delete(&cover).map_err(|error| CoreError::coded("disk", error.to_string()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;

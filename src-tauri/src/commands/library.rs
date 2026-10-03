//! F10: bibliothèque queries and actions; blocking disk/SQL work runs off the UI thread.
use reverb_core::library::{self, LibraryItem, LibraryPage, LibraryQuery};
use reverb_core::CoreError;
use tauri::State;

use crate::state::AppState;
use reverb_core::library::files::{self, ImportReport};
use reverb_core::tagging::{self, TagCover, TrackTags};
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn library_import(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<ImportReport, CoreError> {
    let roots = paths
        .iter()
        .filter_map(|p| files::canonical(std::path::Path::new(p)).ok())
        .collect::<Vec<_>>();
    let mut report = files::import(
        &state.db,
        paths.into_iter().map(PathBuf::from).collect(),
        &state.tools.resolve_ffprobe()?,
        state.sink.clone(),
        "import",
    )
    .await?;
    let settings = state.settings.get();
    if settings.verify_lossless_on_import {
        let _guard = state.tools.acquire_run().await;
        report.failures.extend(
            reverb_core::lossless::verify_imports(
                &state.db,
                roots.clone(),
                &state.tools.resolve(reverb_core::Tool::Ffmpeg)?,
                &state.tools.resolve_ffprobe()?,
            )
            .await?,
        );
        state.sink.emit("library://changed", serde_json::json!({}));
    }
    if !settings.acoustid_key.is_empty() && !settings.offline_mode {
        if state
            .tools
            .resolve(reverb_core::tools::Tool::Fpcalc)
            .is_err()
        {
            state.tools.update(reverb_core::tools::Tool::Fpcalc).await?;
        }
        let _guard = state.tools.acquire_run().await;
        report.failures.extend(
            reverb_core::quality::import::identify(
                &state.db,
                roots,
                &state.tools.resolve(reverb_core::tools::Tool::Fpcalc)?,
                &settings.acoustid_key,
            )
            .await?,
        );
        state.sink.emit("library://changed", serde_json::json!({}));
    }
    Ok(report)
}

#[tauri::command]
pub async fn library_rescan(state: State<'_, AppState>) -> Result<ImportReport, CoreError> {
    let settings = state.settings.get();
    let root = reverb_core::paths::resolve_output_dir(&settings);
    let mut report = files::rescan(
        &state.db,
        reverb_core::paths::resolve_output_dir(&state.settings.get()),
        &state.tools.resolve_ffprobe()?,
        state.sink.clone(),
    )
    .await?;
    if settings.verify_lossless_on_import {
        let _guard = state.tools.acquire_run().await;
        report.failures.extend(
            reverb_core::lossless::verify_imports(
                &state.db,
                vec![root],
                &state.tools.resolve(reverb_core::Tool::Ffmpeg)?,
                &state.tools.resolve_ffprobe()?,
            )
            .await?,
        );
        state.sink.emit("library://changed", serde_json::json!({}));
    }
    Ok(report)
}

#[tauri::command]
pub async fn tags_read(path: String) -> Result<TrackTags, CoreError> {
    tokio::task::spawn_blocking(move || tagging::read_tags(std::path::Path::new(&path)))
        .await
        .map_err(|e| CoreError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn tags_write(
    state: State<'_, AppState>,
    path: String,
    tags: TrackTags,
    reorganize: bool,
) -> Result<String, CoreError> {
    let settings = state.settings.get();
    let result = state
        .db
        .call(move |conn| {
            files::write(
                conn,
                std::path::Path::new(&path),
                &tags,
                reorganize,
                &settings,
            )
        })
        .await?;
    state.sink.emit("library://changed", serde_json::json!({}));
    Ok(result.to_string_lossy().into_owned())
}

async fn pick(
    app: AppHandle,
    images: bool,
    language: reverb_core::settings::Language,
) -> Result<Option<String>, CoreError> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let extensions: &[&str] = if images {
        &["jpg", "jpeg", "png", "webp"]
    } else {
        &["mp3", "m4a", "opus", "ogg", "flac", "wav"]
    };
    app.dialog()
        .file()
        .add_filter(
            match (images, language) {
                (true, reverb_core::settings::Language::PtBr) => "Imagens",
                (false, reverb_core::settings::Language::PtBr) => "Áudio",
                (true, _) => "Images",
                (false, _) => "Audio",
            },
            extensions,
        )
        .pick_file(move |path| {
            let _ = tx.send(path);
        });
    let selected = rx.await.map_err(|e| CoreError::Internal(e.to_string()))?;
    selected
        .map(|path| {
            path.into_path()
                .map(|p| p.to_string_lossy().into_owned())
                .map_err(|e| CoreError::Internal(e.to_string()))
        })
        .transpose()
}

#[tauri::command]
pub async fn pick_audio_file(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, CoreError> {
    pick(app, false, state.settings.get().language).await
}

#[tauri::command]
pub async fn pick_image_file(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, CoreError> {
    pick(app, true, state.settings.get().language).await
}

#[tauri::command]
pub async fn artwork_read(path: String) -> Result<TagCover, CoreError> {
    tokio::task::spawn_blocking(move || {
        let metadata = std::fs::metadata(&path)?;
        if metadata.len() > reverb_core::artwork::MAX_DOWNLOAD_BYTES as u64 {
            return Err(CoreError::coded("artwork_size", "Image exceeds 15 MB"));
        }
        let bytes = std::fs::read(path)?;
        Ok(TagCover {
            mime_type: "image/jpeg".into(),
            data: reverb_core::artwork::process(&bytes)?,
        })
    })
    .await
    .map_err(|e| CoreError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn artwork_fetch(url: String) -> Result<TagCover, CoreError> {
    let cover = reverb_core::artwork::ArtworkClient::default()
        .fetch(&[reverb_core::artwork::CoverCandidate {
            url,
            source: "user".into(),
        }])
        .await
        .ok_or_else(|| CoreError::coded("artwork_decode", "Artwork could not be loaded"))?;
    Ok(TagCover {
        mime_type: "image/jpeg".into(),
        data: cover.jpeg,
    })
}

#[tauri::command]
pub async fn library_list(
    state: State<'_, AppState>,
    query: LibraryQuery,
) -> Result<LibraryPage, CoreError> {
    state.db.call(move |conn| library::list(conn, &query)).await
}

#[tauri::command]
pub async fn library_get(
    state: State<'_, AppState>,
    id: i64,
) -> Result<Option<LibraryItem>, CoreError> {
    state.db.call(move |conn| library::get(conn, id)).await
}

#[tauri::command]
pub async fn library_artists(state: State<'_, AppState>) -> Result<Vec<String>, CoreError> {
    state.db.call(|conn| library::artists(conn)).await
}

#[tauri::command]
pub async fn library_albums(
    state: State<'_, AppState>,
    artist: Option<String>,
) -> Result<Vec<String>, CoreError> {
    state
        .db
        .call(move |conn| library::albums(conn, artist.as_deref()))
        .await
}

#[tauri::command]
pub async fn library_delete(
    state: State<'_, AppState>,
    ids: Vec<i64>,
    delete_files: bool,
) -> Result<usize, CoreError> {
    let result = state
        .db
        .call(move |conn| library::delete(conn, &ids, delete_files))
        .await;
    // Even a partial trash failure may have removed earlier entries.
    state.sink.emit("library://changed", serde_json::json!({}));
    result
}

#[tauri::command]
pub async fn library_clear(state: State<'_, AppState>) -> Result<usize, CoreError> {
    let count = state.db.call(|conn| library::clear(conn)).await?;
    state.sink.emit("library://changed", serde_json::json!({}));
    Ok(count)
}

#[tauri::command]
pub async fn review_dismiss(state: State<'_, AppState>, id: i64) -> Result<(), CoreError> {
    state
        .db
        .call(move |conn| library::dismiss(conn, id))
        .await?;
    state.sink.emit("library://changed", serde_json::json!({}));
    Ok(())
}

#[tauri::command]
pub async fn review_list(
    state: State<'_, AppState>,
    offset: Option<u32>,
) -> Result<LibraryPage, CoreError> {
    state
        .db
        .call(move |conn| {
            library::list(
                conn,
                &LibraryQuery {
                    needs_review: Some(true),
                    offset: offset.unwrap_or(0),
                    ..Default::default()
                },
            )
        })
        .await
}

#[tauri::command]
pub async fn review_apply(
    state: State<'_, AppState>,
    id: i64,
    candidate: Option<reverb_core::metadata::Candidate>,
    manual: Option<TrackTags>,
) -> Result<LibraryItem, CoreError> {
    if candidate.is_some() == manual.is_some() {
        return Err(CoreError::invalid("Choose a candidate or manual tags"));
    }
    let item = state
        .db
        .call(move |conn| library::get(conn, id))
        .await?
        .ok_or_else(|| CoreError::coded("not_found", "Library item missing"))?;
    let path = item.file_path.clone();
    let mut tags = if let Some(tags) = manual {
        tags
    } else {
        tokio::task::spawn_blocking(move || tagging::read_tags(std::path::Path::new(&path)))
            .await
            .map_err(|e| CoreError::Internal(e.to_string()))??
    };
    if let Some(candidate) = &candidate {
        tags.title = candidate.title.clone();
        if !candidate.artists.is_empty() {
            tags.artist = Some(candidate.artists.join(", "));
        }
        tags.album = candidate.album.clone().or(tags.album);
        tags.album_artist = candidate.album_artist.clone().or(tags.album_artist);
        tags.year = candidate.year.or(tags.year);
        tags.genre = candidate.genre.clone().or(tags.genre);
        tags.track_no = candidate.track_no.or(tags.track_no);
        tags.track_total = candidate.track_total.or(tags.track_total);
        tags.disc_no = candidate.disc_no.or(tags.disc_no);
        tags.isrc = candidate.isrc.clone().or(tags.isrc);
        if let Some(url) = &candidate.cover_url {
            tags.cover = Some(artwork_fetch(url.clone()).await?);
        }
    }
    let settings = state.settings.get();
    let source = candidate
        .map(|c| c.provider)
        .unwrap_or_else(|| "user".into());
    let result = state
        .db
        .call(move |conn| files::apply_review(conn, id, &tags, &settings, &source))
        .await?;
    state.sink.emit("library://changed", serde_json::json!({}));
    Ok(result)
}

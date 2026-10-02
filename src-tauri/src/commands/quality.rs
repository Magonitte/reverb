use crate::state::AppState;
use reverb_core::{quality, tools::Tool, CoreError};
use std::path::PathBuf;
use tauri::State;
use tokio_util::sync::CancellationToken;

#[tauri::command]
pub async fn upgrade_scan(
    state: State<'_, AppState>,
    ids: Option<Vec<i64>>,
) -> Result<Vec<quality::upgrade::UpgradeCandidate>, CoreError> {
    quality::upgrade::scan(
        &state.db,
        state.backend.as_ref(),
        ids,
        &CancellationToken::new(),
    )
    .await
}
#[tauri::command]
pub async fn upgrade_enqueue(
    state: State<'_, AppState>,
    ids: Vec<i64>,
) -> Result<Vec<reverb_core::queue::Job>, CoreError> {
    let candidates = quality::upgrade::scan(
        &state.db,
        state.backend.as_ref(),
        Some(ids),
        &CancellationToken::new(),
    )
    .await?;
    let mut jobs = Vec::new();
    for candidate in candidates {
        let item = candidate.item;
        let request = reverb_core::queue::EnqueueRequest {
            url: item
                .source_url
                .ok_or_else(|| CoreError::invalid("Source missing"))?,
            source_id: item.source_id,
            title: Some(item.title),
            profile_id: item.profile_id,
            options: Some(reverb_core::queue::JobOptions {
                upgrade_library_id: Some(item.id),
                ..Default::default()
            }),
            allow_duplicate: true,
            ..Default::default()
        };
        jobs.push(state.queue.enqueue(request).await?);
    }
    Ok(jobs)
}
#[tauri::command]
pub async fn provider_test(state: State<'_, AppState>, provider: String) -> Result<(), CoreError> {
    if provider == "acoustid" {
        let key = state.settings.get().acoustid_key;
        if key.is_empty() {
            return Err(CoreError::coded("provider_key", "AcoustID key missing"));
        }
        if state.tools.resolve(Tool::Fpcalc).is_err() {
            state.tools.update(Tool::Fpcalc).await?;
        }
        let _guard = state.tools.acquire_run().await;
        let dir = tempfile::tempdir()?;
        let audio = dir.path().join("test.wav");
        quality::audio::execute(
            &state.tools.resolve(Tool::Ffmpeg)?,
            vec![
                "-nostdin".into(),
                "-y".into(),
                "-f".into(),
                "lavfi".into(),
                "-i".into(),
                "sine=frequency=440:duration=15".into(),
                audio.to_string_lossy().into_owned(),
            ],
            &CancellationToken::new(),
        )
        .await
        .map_err(crate::commands::media::engine_error)?;
        quality::acoustid::lookup(
            &state.tools.resolve(Tool::Fpcalc)?,
            &audio,
            &key,
            "https://api.acoustid.org/v2/lookup",
        )
        .await?;
        return Ok(());
    }
    state.metadata.keyed.test(&provider).await
}

#[tauri::command]
pub async fn waveform(state: State<'_, AppState>, path: String) -> Result<String, CoreError> {
    let _guard = state.tools.acquire_run().await;
    let bytes = quality::audio::waveform(
        &state.tools.resolve(Tool::Ffmpeg)?,
        &PathBuf::from(path),
        &CancellationToken::new(),
    )
    .await
    .map_err(crate::commands::media::engine_error)?;
    Ok(quality::png_url(&bytes))
}
#[tauri::command]
pub async fn trim_audio(
    state: State<'_, AppState>,
    path: String,
    start_s: f64,
    end_s: f64,
) -> Result<(), CoreError> {
    let _guard = state.tools.acquire_run().await;
    quality::trim(
        &state.db,
        &state.tools.resolve(Tool::Ffmpeg)?,
        &state.tools.resolve_ffprobe()?,
        &PathBuf::from(path),
        start_s,
        end_s,
        &CancellationToken::new(),
    )
    .await?;
    state.sink.emit("library://changed", serde_json::json!({}));
    Ok(())
}

#[tauri::command]
pub async fn audio_duration(state: State<'_, AppState>, path: String) -> Result<f64, CoreError> {
    let _guard = state.tools.acquire_run().await;
    let probe =
        reverb_core::transcode::probe(&state.tools.resolve_ffprobe()?, &PathBuf::from(path))
            .await
            .map_err(crate::commands::media::engine_error)?;
    Ok(probe.duration_s)
}

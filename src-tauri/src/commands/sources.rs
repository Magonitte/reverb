use crate::state::AppState;
use reverb_core::{lossless::LosslessReport, CoreError};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub async fn verify_lossless(
    state: State<'_, AppState>,
    id: i64,
) -> Result<LosslessReport, CoreError> {
    let _guard = state.tools.acquire_run().await;
    let report = reverb_core::lossless::verify_library(
        &state.db,
        &state.tools.resolve(reverb_core::Tool::Ffmpeg)?,
        &state.tools.resolve_ffprobe()?,
        id,
    )
    .await?;
    state.sink.emit("library://changed", serde_json::json!({}));
    Ok(report)
}
#[tauri::command]
pub fn open_source_url(app: AppHandle, url: String) -> Result<(), CoreError> {
    if reverb_core::sources::matches(&url).is_none()
        && reverb_core::import::classify(&url).is_none()
        && url != "https://developer.spotify.com/dashboard"
    {
        return Err(CoreError::invalid("Unsupported source URL"));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| CoreError::Internal(e.to_string()))
}

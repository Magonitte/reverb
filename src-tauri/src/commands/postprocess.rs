use crate::state::AppState;
use reverb_core::{library, CoreError};
use tauri::State;

#[tauri::command]
pub fn template_preview(state: State<'_, AppState>, template: String) -> Result<String, CoreError> {
    reverb_core::organize::template::preview(&template, &state.settings.get())
}

#[tauri::command]
pub async fn library_cover(
    state: State<'_, AppState>,
    id: i64,
) -> Result<Option<String>, CoreError> {
    let item = state
        .db
        .call(move |conn| library::get(conn, id))
        .await?
        .ok_or_else(|| CoreError::coded("file_missing", "Registro da biblioteca não encontrado"))?;
    tokio::task::spawn_blocking(move || {
        library::cover_thumbnail(std::path::Path::new(&item.file_path))
    })
    .await
    .map_err(|e| CoreError::Internal(e.to_string()))?
}

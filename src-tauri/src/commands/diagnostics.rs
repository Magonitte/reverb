use crate::state::AppState;
use reverb_core::{desktop::diagnostics::DiagnosticReport, CoreError};
use tauri::AppHandle;
use tauri::State;

#[tauri::command]
pub async fn diagnostics_last(
    state: State<'_, AppState>,
) -> Result<Option<DiagnosticReport>, CoreError> {
    state
        .db
        .kv_get("diagnostics.last")
        .await?
        .map(|value| serde_json::from_str(&value).map_err(CoreError::from))
        .transpose()
}

#[tauri::command]
pub async fn diagnostics_run(app: AppHandle) -> Result<DiagnosticReport, CoreError> {
    let _guard = crate::diagnostics::GATE.lock().await;
    crate::diagnostics::run(&app).await
}

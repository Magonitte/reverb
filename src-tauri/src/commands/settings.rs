use reverb_core::{CoreError, SettingsPatch, SettingsView};
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub async fn settings_get(state: State<'_, AppState>) -> Result<SettingsView, CoreError> {
    Ok(state.settings.view())
}

#[tauri::command]
pub async fn settings_update(
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> Result<SettingsView, CoreError> {
    Ok(state.settings.update(patch).await?.view())
}

#[tauri::command]
pub async fn settings_reset(state: State<'_, AppState>) -> Result<SettingsView, CoreError> {
    Ok(state.settings.reset().await?.view())
}

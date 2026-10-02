use reverb_core::{CoreError, SettingsPatch, SettingsView};
use tauri::{AppHandle, State};

use crate::state::AppState;

#[tauri::command]
pub async fn settings_get(state: State<'_, AppState>) -> Result<SettingsView, CoreError> {
    Ok(state.settings.view())
}

#[tauri::command]
pub async fn settings_update(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: SettingsPatch,
) -> Result<SettingsView, CoreError> {
    let _guard = crate::startup::SETTINGS_EFFECTS.lock().await;
    let previous = state.settings.get();
    let mut next = previous.clone();
    next.apply(patch.clone());
    reverb_core::settings::validate(&next)?;
    if previous.launch_at_startup != next.launch_at_startup {
        crate::startup::apply(&app, next.launch_at_startup)?;
    }
    let view = match state.settings.update(patch).await {
        Ok(settings) => settings.view(),
        Err(error) => {
            if previous.launch_at_startup != next.launch_at_startup {
                if let Err(rollback) = crate::startup::apply(&app, previous.launch_at_startup) {
                    tracing::error!(kind = rollback.kind(), "autostart rollback failed");
                }
            }
            return Err(error);
        }
    };
    // `parallelism` vale em tempo real: a fila reavalia quantos jobs pode rodar.
    state.queue.wake();
    Ok(view)
}

#[tauri::command]
pub async fn settings_reset(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SettingsView, CoreError> {
    let _guard = crate::startup::SETTINGS_EFFECTS.lock().await;
    let previous = state.settings.get();
    crate::startup::apply(&app, false)?;
    match state.settings.reset().await {
        Ok(settings) => {
            state.queue.wake();
            Ok(settings.view())
        }
        Err(error) => {
            if let Err(rollback) = crate::startup::apply(&app, previous.launch_at_startup) {
                tracing::error!(kind = rollback.kind(), "autostart rollback failed");
            }
            Err(error)
        }
    }
}

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
    let manual_channel = patch.ytdlp_channel.is_some();
    let previous = state.settings.get();
    let mut next = previous.clone();
    next.apply(patch.clone());
    reverb_core::settings::validate(&next)?;
    if next.onboarding_completed && !previous.onboarding_completed {
        state.tools.resolve(reverb_core::Tool::Ytdlp)?;
        state.tools.resolve(reverb_core::Tool::Ffmpeg)?;
        let runtime = reverb_core::tools::detect_js_runtime(
            next.js_runtime,
            &std::env::var_os("PATH").unwrap_or_default(),
            state.tools.resolve(reverb_core::Tool::Deno).ok(),
        )
        .await?;
        if !matches!(runtime, reverb_core::tools::Detection::Found(_)) {
            return Err(CoreError::invalid_i18n(
                "Install the required tools before completing onboarding",
                "onboarding.toolsRequired",
            ));
        }
    }
    crate::startup::apply_settings(&app, &previous, &next)?;
    let view = match state.settings.update(patch).await {
        Ok(settings) => settings.view(),
        Err(error) => {
            if let Err(rollback) = crate::startup::apply_settings(&app, &next, &previous) {
                tracing::error!(kind = rollback.kind(), "native settings rollback failed");
            }
            return Err(error);
        }
    };
    // `parallelism` vale em tempo real: a fila reavalia quantos jobs pode rodar.
    state.queue.wake();
    if manual_channel {
        state
            .db
            .call(|conn| {
                conn.execute("DELETE FROM kv WHERE key='heal_switched_to_nightly'", [])?;
                Ok(())
            })
            .await?;
    }
    Ok(view)
}

#[tauri::command]
pub async fn settings_reset(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SettingsView, CoreError> {
    let _guard = crate::startup::SETTINGS_EFFECTS.lock().await;
    let previous = state.settings.get();
    let next = reverb_core::Settings::default();
    crate::startup::apply_settings(&app, &previous, &next)?;
    match state.settings.reset().await {
        Ok(settings) => {
            state.queue.wake();
            Ok(settings.view())
        }
        Err(error) => {
            if let Err(rollback) = crate::startup::apply_settings(&app, &next, &previous) {
                tracing::error!(kind = rollback.kind(), "native settings rollback failed");
            }
            Err(error)
        }
    }
}

use reverb_core::tools::{InstallOutcome, ToolStatus, UpdateInfo};
use reverb_core::{CoreError, Tool};
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub async fn runtime_choices(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<reverb_core::settings::JsRuntime>, reverb_core::CoreError> {
    use reverb_core::settings::JsRuntime;
    use reverb_core::tools::{detect_js_runtime, Detection, Tool};
    let mut choices = Vec::new();
    for mode in [
        JsRuntime::ManagedDeno,
        JsRuntime::SystemDeno,
        JsRuntime::SystemNode,
    ] {
        if matches!(
            detect_js_runtime(
                mode,
                &std::env::var_os("PATH").unwrap_or_default(),
                state.tools.resolve(Tool::Deno).ok()
            )
            .await,
            Ok(Detection::Found(_))
        ) {
            choices.push(mode);
        }
    }
    Ok(choices)
}

#[tauri::command]
pub async fn tools_status(state: State<'_, AppState>) -> Result<Vec<ToolStatus>, CoreError> {
    state.tools.status().await
}

/// Instala o que faltar (yt-dlp, FFmpeg, runtime JS). Devolve as ferramentas instaladas agora.
#[tauri::command]
pub async fn tools_install_missing(state: State<'_, AppState>) -> Result<Vec<Tool>, CoreError> {
    state.tools.install_missing().await
}

#[tauri::command]
pub async fn tools_check_updates(
    state: State<'_, AppState>,
    force: Option<bool>,
) -> Result<Vec<UpdateInfo>, CoreError> {
    state.tools.check_updates(force.unwrap_or(false)).await
}

#[tauri::command]
pub async fn tools_update(
    state: State<'_, AppState>,
    tool: Tool,
) -> Result<InstallOutcome, CoreError> {
    state.tools.update(tool).await
}

/// Volta para a versão anterior; devolve a versão que ficou ativa.
#[tauri::command]
pub async fn tools_rollback(state: State<'_, AppState>, tool: Tool) -> Result<String, CoreError> {
    state.tools.rollback(tool).await
}

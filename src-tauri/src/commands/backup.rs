use crate::state::AppState;
use reverb_core::{
    desktop::{backup, diagnostics::DiagnosticReport},
    CoreError,
};
use std::path::Path;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub async fn pick_backup_path(app: AppHandle, restore: bool) -> Result<Option<String>, CoreError> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let dialog = app.dialog().file().add_filter("ZIP", &["zip"]);
    if restore {
        dialog.pick_file(move |path| {
            let _ = tx.send(path);
        });
    } else {
        dialog
            .set_file_name("reverb-backup.zip")
            .save_file(move |path| {
                let _ = tx.send(path);
            });
    }
    rx.await
        .map_err(|_| CoreError::Internal("File dialog closed".into()))?
        .map(|path| {
            path.into_path()
                .map(|path| path.to_string_lossy().into_owned())
                .map_err(|error| CoreError::Internal(error.to_string()))
        })
        .transpose()
}

#[tauri::command]
pub async fn logs_export(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, CoreError> {
    let Some(path) = pick_backup_path(app.clone(), false).await? else {
        return Ok(None);
    };
    let report: DiagnosticReport = match state.db.kv_get("diagnostics.last").await? {
        Some(value) => serde_json::from_str(&value)?,
        None => DiagnosticReport {
            created_at: reverb_core::queue::repo::now(),
            items: vec![],
        },
    };
    backup::logs_export(&state.paths.logs_dir(), &report, Path::new(&path)).await?;
    Ok(Some(path))
}

#[tauri::command]
pub async fn data_export(state: State<'_, AppState>, path: String) -> Result<(), CoreError> {
    let _guard = crate::startup::SETTINGS_EFFECTS.lock().await;
    if Path::new(&path) == state.paths.db_file() {
        return Err(CoreError::invalid("Choose a backup ZIP destination"));
    }
    backup::data_export(&state.db, &state.settings.get(), Path::new(&path)).await
}

#[tauri::command]
pub async fn data_import(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<String, CoreError> {
    let _settings = crate::startup::SETTINGS_EFFECTS.lock().await;
    let _diagnostics = crate::diagnostics::GATE.lock().await;
    let source = backup::prepare_import(Path::new(&path)).await?;
    // Initial tool maintenance must finish before replacing its shared database.
    if let Some(task) = state.tools_startup.lock().await.take() {
        task.await
            .map_err(|error| CoreError::Internal(error.to_string()))?;
    }
    state.background_cancel.cancel();
    let _sync = state.syncs.stop_and_wait().await;
    state.queue.shutdown().await;
    let result = match state.queue.state().await {
        Ok(queue) if queue.running == 0 && !queue.healing => {
            backup::restore(
                &state.db,
                &state.settings.get(),
                &state.paths.data_dir,
                source,
            )
            .await
        }
        Ok(_) => Err(CoreError::coded(
            "busy",
            "Background download work has not stopped",
        )),
        Err(error) => Err(error),
    };
    // Both success and failure restart: stopped services and settings cache must be reloaded.
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        app.restart();
    });
    result.map(|path| path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn open_data_dir(app: AppHandle, state: State<'_, AppState>) -> Result<(), CoreError> {
    app.opener()
        .open_path(state.paths.data_dir.to_string_lossy(), None::<&str>)
        .map_err(|error| CoreError::Internal(error.to_string()))
}

#[tauri::command]
pub fn data_paths(state: State<'_, AppState>) -> reverb_core::DataPaths {
    state.paths.clone()
}

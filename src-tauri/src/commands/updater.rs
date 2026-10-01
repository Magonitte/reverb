use tauri::AppHandle;

use crate::updater::{self, AppUpdateInfo};

#[tauri::command]
pub async fn updater_check(app: AppHandle) -> Result<Option<AppUpdateInfo>, String> {
    updater::check(&app).await
}

#[tauri::command]
pub async fn updater_install(app: AppHandle) -> Result<(), String> {
    updater::install(&app, true).await
}

#[tauri::command]
pub fn app_restart(app: AppHandle) {
    app.restart();
}

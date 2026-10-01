pub mod queue;
pub mod settings;
pub mod tools;
pub mod updater;

use reverb_core::AppInfo;
use tauri::AppHandle;

#[tauri::command]
pub fn app_info(app: AppHandle) -> AppInfo {
    AppInfo::new(app.package_info().version.to_string())
}

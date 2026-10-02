use reverb_core::{CoreError, CoreResult};
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

pub const BOOKMARKLET: &str =
    "javascript:location.href='reverb://add?url='+encodeURIComponent(location.href)";

#[tauri::command]
pub fn bookmarklet_code() -> String {
    BOOKMARKLET.to_owned()
}

#[tauri::command]
pub fn bookmarklet_copy(app: AppHandle) -> CoreResult<()> {
    app.clipboard()
        .write_text(BOOKMARKLET)
        .map_err(|error| CoreError::Internal(error.to_string()))
}

#[tauri::command]
pub async fn deeplink_test(app: AppHandle, link: String) -> CoreResult<()> {
    crate::integration::receive(&app, &link).await
}

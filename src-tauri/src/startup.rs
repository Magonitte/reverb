use reverb_core::{CoreError, CoreResult};
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

pub static SETTINGS_EFFECTS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub fn apply_settings(
    app: &AppHandle,
    before: &reverb_core::Settings,
    next: &reverb_core::Settings,
) -> CoreResult<()> {
    crate::shortcut::apply(app, &before.global_shortcut, &next.global_shortcut)?;
    if before.launch_at_startup != next.launch_at_startup {
        if let Err(error) = apply(app, next.launch_at_startup) {
            if let Err(rollback) =
                crate::shortcut::apply(app, &next.global_shortcut, &before.global_shortcut)
            {
                tracing::error!(kind = rollback.kind(), "shortcut rollback failed");
            }
            return Err(error);
        }
    }
    Ok(())
}

pub fn app_name() -> String {
    if cfg!(debug_assertions) {
        if let Some(dir) = std::env::var_os("REVERB_DATA_DIR") {
            let path = std::path::PathBuf::from(dir);
            let name = path
                .parent()
                .and_then(|path| path.file_name())
                .unwrap_or_default()
                .to_string_lossy();
            return format!(
                "Reverb-test-{}",
                name.replace(|c: char| !c.is_ascii_alphanumeric(), "")
            );
        }
    }
    "Reverb".into()
}

pub fn apply(app: &AppHandle, enabled: bool) -> CoreResult<()> {
    let result = if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    };
    result.map_err(|_| {
        CoreError::invalid_i18n("Could not update autostart", "integration.autostartFailed")
    })
}

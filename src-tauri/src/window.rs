use crate::state::AppState;
use tauri::{Manager, WindowEvent};

pub fn initialize(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let current = window.clone();
    window.on_window_event(move |event| {
        let settings = current.state::<AppState>().settings.get();
        let hide = match event {
            WindowEvent::CloseRequested { api, .. } if settings.close_to_tray => {
                api.prevent_close();
                true
            }
            WindowEvent::Resized(_) if settings.minimize_to_tray => {
                current.is_minimized().unwrap_or(false)
            }
            _ => false,
        };
        if hide {
            if let Err(error) = current.hide() {
                tracing::warn!(%error, "could not hide main window");
            }
        }
    });
}

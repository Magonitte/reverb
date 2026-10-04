use reverb_core::desktop::texts;
use reverb_core::integration::{add_link, supported_url, ClipboardDedup};
use reverb_core::{CoreError, CoreResult};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_opener::OpenerExt;

use crate::{integration, state::AppState};

struct TrayControls {
    tray: TrayIcon,
    header: MenuItem<tauri::Wry>,
    open: MenuItem<tauri::Wry>,
    pause: MenuItem<tauri::Wry>,
    clipboard: MenuItem<tauri::Wry>,
    folder: MenuItem<tauri::Wry>,
    exit: MenuItem<tauri::Wry>,
}

pub async fn download_clipboard(app: &AppHandle) -> CoreResult<()> {
    let text = app
        .clipboard()
        .read_text()
        .map_err(|_| CoreError::invalid("clipboard has no text"))?;
    integration::receive(app, &add_link(&text)?).await
}

pub fn initialize(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let language = app.state::<AppState>().settings.get().language;
    let item =
        |id, enabled| MenuItem::with_id(app, id, texts::text(language, id), enabled, None::<&str>);
    let controls = (
        MenuItem::with_id(
            app,
            "header",
            texts::active(language, 0),
            false,
            None::<&str>,
        )?,
        item("open", true)?,
        item("pause", true)?,
        item("clipboard", false)?,
        item("folder", true)?,
        item("exit", true)?,
    );
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &controls.0,
            &separator,
            &controls.1,
            &controls.2,
            &controls.3,
            &controls.4,
            &controls.5,
        ],
    )?;
    let icon = app
        .default_window_icon()
        .ok_or("missing application tray icon")?
        .clone();
    let tray = TrayIconBuilder::with_id("reverb")
        .icon(icon)
        .tooltip(texts::active(language, 0))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                integration::show_window(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| {
            let action = event.id.as_ref();
            if action == "open" {
                integration::show_window(app);
                return;
            }
            if action == "exit" {
                app.exit(0);
                return;
            }
            let action = action.to_owned();
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                let result = match action.as_str() {
                    "pause" => match state.queue.state().await {
                        Ok(queue) => {
                            if queue.paused {
                                state.queue.resume().await;
                            } else {
                                state.queue.pause().await;
                            }
                            Ok(())
                        }
                        Err(error) => Err(error),
                    },
                    "clipboard" => download_clipboard(&app).await,
                    "folder" => {
                        let dir = reverb_core::paths::resolve_output_dir(&state.settings.get());
                        match std::fs::create_dir_all(&dir) {
                            Ok(()) => app
                                .opener()
                                .open_path(dir.to_string_lossy(), None::<&str>)
                                .map_err(|error| CoreError::Internal(error.to_string())),
                            Err(error) => Err(error.into()),
                        }
                    }
                    _ => Ok(()),
                };
                if let Err(error) = result {
                    tracing::warn!(kind = error.kind(), "tray action failed");
                    state.sink.emit(
                        "notice",
                        serde_json::json!({"level":"error","i18nKey":"integration.linkFailed"}),
                    );
                }
            });
        })
        .build(app)?;
    app.manage(TrayControls {
        tray,
        header: controls.0,
        open: controls.1,
        pause: controls.2,
        clipboard: controls.3,
        folder: controls.4,
        exit: controls.5,
    });
    let handle = app.clone();
    let cancel = app.state::<AppState>().background_cancel.clone();
    tauri::async_runtime::spawn(async move {
        let mut watcher = ClipboardDedup::default();
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(1500));
        loop {
            tokio::select! { _ = cancel.cancelled() => break, _ = interval.tick() => {} }
            match refresh(&handle).await {
                Ok(copied) => {
                    let state = handle.state::<AppState>();
                    if state.settings.get().clipboard_watch {
                        if let Some(url) = copied.and_then(|text| watcher.observe(&text)) {
                            let visible = handle
                                .get_webview_window("main")
                                .map(|window| {
                                    window.is_focused().unwrap_or(false)
                                        && !window.is_minimized().unwrap_or(false)
                                })
                                .unwrap_or(false);
                            state.sink.emit(
                                "clipboard://url",
                                serde_json::json!({"url":url.clone(),"visible":visible}),
                            );
                            if !visible {
                                crate::notifications::show_clipboard(&handle, url);
                            }
                        }
                    } else {
                        watcher = ClipboardDedup::default();
                    }
                }
                Err(error) => tracing::debug!(%error, "tray refresh failed"),
            }
        }
    });
    Ok(())
}

async fn refresh(app: &AppHandle) -> CoreResult<Option<String>> {
    let state = app.state::<AppState>();
    let language = state.settings.get().language;
    let queue = state.queue.state().await?;
    let controls = app.state::<TrayControls>();
    let title = texts::active(language, (queue.running + queue.queued) as usize);
    let clipboard = match app.clipboard().read_text() {
        Ok(text) => Some(text),
        Err(error) => {
            tracing::debug!(%error, "clipboard read unavailable");
            None
        }
    };
    let copied = clipboard.as_deref().and_then(supported_url).is_some();
    let apply = || -> tauri::Result<()> {
        controls.header.set_text(&title)?;
        controls.tray.set_tooltip(Some(&title))?;
        controls.open.set_text(texts::text(language, "open"))?;
        controls.pause.set_text(texts::text(
            language,
            if queue.paused { "resume" } else { "pause" },
        ))?;
        controls
            .clipboard
            .set_text(texts::text(language, "clipboard"))?;
        controls.clipboard.set_enabled(copied)?;
        controls.folder.set_text(texts::text(language, "folder"))?;
        controls.exit.set_text(texts::text(language, "exit"))?;
        Ok(())
    };
    apply().map_err(|error| CoreError::Internal(error.to_string()))?;
    Ok(clipboard)
}

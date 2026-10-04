use crate::state::AppState;
use reverb_core::desktop::{notifications::CompletionBatch, texts};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_notification::NotificationExt;

#[derive(Clone, serde::Serialize)]
pub struct NativeNotice {
    message: String,
    title: String,
    tone: String,
    url: Option<String>,
}
#[derive(Default)]
pub struct PendingNotices(Mutex<Vec<NativeNotice>>);

pub fn show(app: &AppHandle, message: &str) {
    show_rich(
        app,
        NativeNotice {
            message: message.to_owned(),
            title: "Reverb".into(),
            tone: "info".into(),
            url: None,
        },
    );
}
pub fn show_clipboard(app: &AppHandle, url: String) {
    show_rich(
        app,
        NativeNotice {
            message: format!(
                "{} — {}",
                texts::text(app.state::<AppState>().settings.get().language, "copied"),
                url
            ),
            title: "Reverb".into(),
            tone: "info".into(),
            url: Some(url),
        },
    );
}
fn show_rich(app: &AppHandle, notice: NativeNotice) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_focused().unwrap_or(false) && !window.is_minimized().unwrap_or(false) {
            let _ = window.emit("notification://notice", &notice);
            return;
        }
    }
    if let Some(pending) = app.try_state::<PendingNotices>() {
        {
            let mut items = pending.0.lock().unwrap();
            // The latest clipboard link replaces a previous undecided link.
            if notice.url.is_some() {
                items.retain(|item| item.url.is_none());
            }
            items.push(notice.clone());
        }
        let window = app
            .get_webview_window("notification")
            .map(Ok)
            .unwrap_or_else(|| {
                WebviewWindowBuilder::new(
                    app,
                    "notification",
                    WebviewUrl::App("index.html#/notification".into()),
                )
                .title("Reverb")
                .data_directory(app.state::<AppState>().paths.data_dir.join("webview"))
                .inner_size(452.0, 420.0)
                .decorations(false)
                .transparent(true)
                .background_color(tauri::window::Color(0, 0, 0, 0))
                .shadow(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .focused(false)
                .visible(false)
                .build()
            });
        match window {
            Ok(window) => {
                let _ = window.emit("notification://pending", ());
                return;
            }
            Err(error) => tracing::warn!(%error, "notification popup unavailable"),
        }
    }
    if let Err(error) = app
        .notification()
        .builder()
        .title(&notice.title)
        .body(&notice.message)
        .show()
    {
        tracing::warn!(%error, "native notification failed");
    }
}
#[tauri::command]
pub fn notifications_pending(app: AppHandle) -> Vec<NativeNotice> {
    app.try_state::<PendingNotices>()
        .map(|pending| std::mem::take(&mut *pending.0.lock().unwrap()))
        .unwrap_or_default()
}
#[tauri::command]
pub fn notifications_hide(app: AppHandle) {
    if let Some(window) = app.get_webview_window("notification") {
        let _ = window.hide();
    }
}
#[tauri::command]
pub fn notifications_resize(app: AppHandle, height: u32) -> Result<(), String> {
    let Some(window) = app.get_webview_window("notification") else {
        return Ok(());
    };
    if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
        let scale = monitor.scale_factor();
        let area = monitor.work_area();
        let width = 452.0_f64.min((area.size.width as f64 / scale - 32.0).max(1.0));
        let height =
            (height.clamp(160, 640) as f64).min((area.size.height as f64 / scale - 32.0).max(1.0));
        window
            .set_size(tauri::LogicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
        window
            .set_position(tauri::PhysicalPosition::new(
                area.position.x + area.size.width as i32 - ((width + 16.0) * scale).round() as i32,
                area.position.y + area.size.height as i32
                    - ((height + 16.0) * scale).round() as i32,
            ))
            .map_err(|e| e.to_string())?;
    }
    window.show().map_err(|e| e.to_string())
}

pub fn initialize(app: &AppHandle) -> reverb_core::CoreResult<()> {
    app.manage(PendingNotices::default());
    let app_artists = app.clone();
    app.listen("artists://release", move |event| {
        if !app_artists
            .state::<AppState>()
            .settings
            .get()
            .completion_notifications
        {
            return;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(event.payload()) {
            if let (Some(artist), Some(title)) = (v["artist"].as_str(), v["title"].as_str()) {
                let language = app_artists.state::<AppState>().settings.get().language;
                show(
                    &app_artists,
                    &if language == reverb_core::settings::Language::En {
                        format!("New release by {artist}: {title}")
                    } else {
                        format!("Novo lançamento de {artist}: {title}")
                    },
                );
            }
        }
    });
    let start = Instant::now();
    let batch = Arc::new(Mutex::new(CompletionBatch::default()));
    let finished = Arc::new(Mutex::new(HashSet::new()));
    for job in tauri::async_runtime::block_on(app.state::<AppState>().queue.list())? {
        if job.status.is_finished() {
            finished.lock().unwrap().insert(job.id);
        }
    }
    let app_jobs = app.clone();
    let job_batch = batch.clone();
    app.listen("job://updated", move |event| {
        let Ok(job) = serde_json::from_str::<serde_json::Value>(event.payload()) else {
            return;
        };
        let (Some(id), Some(status)) = (job["id"].as_str(), job["status"].as_str()) else {
            return;
        };
        if !matches!(status, "done" | "failed" | "cancelled") {
            finished.lock().unwrap().remove(id);
            return;
        }
        if !finished.lock().unwrap().insert(id.to_owned()) {
            return;
        }
        let settings = app_jobs.state::<AppState>().settings.get();
        if status == "done" && settings.completion_notifications {
            job_batch.lock().unwrap().push(
                start.elapsed(),
                job["title"].as_str().unwrap_or("Reverb").to_owned(),
            );
        } else if status == "failed" {
            show_rich(
                &app_jobs,
                NativeNotice {
                    title: if settings.language == reverb_core::settings::Language::En {
                        "Download failed"
                    } else {
                        "Falha no download"
                    }
                    .into(),
                    message: format!(
                        "{} · {}",
                        job["title"].as_str().unwrap_or("Reverb"),
                        texts::text(settings.language, "failed")
                    ),
                    tone: "error".into(),
                    url: None,
                },
            );
        }
    });
    let handle = app.clone();
    let cancel = app.state::<AppState>().background_cancel.clone();
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(250));
        loop {
            tokio::select! { _ = cancel.cancelled() => break, _ = ticker.tick() => {} }
            let settings = handle.state::<AppState>().settings.get();
            let messages = {
                let mut batch = batch.lock().unwrap();
                if settings.completion_notifications {
                    batch.flush(start.elapsed(), settings.language)
                } else {
                    batch.clear();
                    vec![]
                }
            };
            for message in messages {
                show_rich(
                    &handle,
                    NativeNotice {
                        title: if settings.language == reverb_core::settings::Language::En {
                            "Download complete"
                        } else {
                            "Download concluído"
                        }
                        .into(),
                        message,
                        tone: "success".into(),
                        url: None,
                    },
                );
            }
        }
    });
    let handle = app.clone();
    let stages = Arc::new(Mutex::new(String::new()));
    app.listen("heal://state", move |event| {
        let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) else {
            return;
        };
        let Some(stage) = payload["stage"].as_str() else {
            return;
        };
        let mut previous = stages.lock().unwrap();
        let language = handle.state::<AppState>().settings.get().language;
        if stage == "failed" && previous.as_str() != "failed" {
            show(&handle, texts::text(language, "healFailed"));
        } else if stage != "done"
            && stage != "failed"
            && matches!(previous.as_str(), "" | "done" | "failed")
        {
            show(&handle, texts::text(language, "heal"));
        }
        *previous = stage.to_owned();
    });
    let handle = app.clone();
    app.listen("updater://available", move |_| {
        show(
            &handle,
            texts::text(handle.state::<AppState>().settings.get().language, "update"),
        );
    });
    let seen = Arc::new(Mutex::new(HashSet::new()));
    let handle = app.clone();
    app.listen("sync://updated", move |event| {
        let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) else {
            return;
        };
        let Some(id) = payload["id"].as_str().map(str::to_owned) else {
            return;
        };
        let handle = handle.clone();
        let seen = seen.clone();
        tauri::async_runtime::spawn(async move {
            let state = handle.state::<AppState>();
            if let Ok(syncs) = state.syncs.list().await {
                if let Some(sync) = syncs.into_iter().find(|sync| sync.id == id) {
                    if let Some(result) = sync.last_result.filter(|result| !result.running) {
                        let stamp = (id, sync.last_sync_at);
                        if !seen.lock().unwrap().insert(stamp) {
                            return;
                        }
                        show(
                            &handle,
                            &texts::sync_summary(
                                state.settings.get().language,
                                &sync.title,
                                result.added,
                                result.failed,
                            ),
                        );
                    }
                }
            }
        });
    });
    Ok(())
}

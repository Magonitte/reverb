use crate::state::AppState;
use reverb_core::desktop::{notifications::CompletionBatch, texts};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Listener, Manager};
use tauri_plugin_notification::NotificationExt;

pub fn show(app: &AppHandle, message: &str) {
    if let Err(error) = app
        .notification()
        .builder()
        .title("Reverb")
        .body(message)
        .show()
    {
        tracing::warn!(%error, "native notification failed");
    }
}

pub fn initialize(app: &AppHandle) -> reverb_core::CoreResult<()> {
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
            show(&app_jobs, texts::text(settings.language, "failed"));
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
                show(&handle, &message);
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

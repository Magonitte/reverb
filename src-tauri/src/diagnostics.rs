use crate::state::AppState;
use reverb_core::desktop::{diagnostics, texts};
use reverb_core::{CoreResult, Tool};
use std::time::Duration;
use tauri::{AppHandle, Manager};

pub static GATE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub async fn run(app: &AppHandle) -> CoreResult<diagnostics::DiagnosticReport> {
    let state = app.state::<AppState>();
    diagnostics::run(&state.db, state.settings.clone(), state.tools.clone()).await
}

async fn weekly(app: &AppHandle) -> CoreResult<()> {
    let _guard = GATE.lock().await;
    let state = app.state::<AppState>();
    if !state.settings.get().weekly_self_test || !state.settings.get().onboarding_completed {
        return Ok(());
    }
    let last = state
        .db
        .kv_get("diagnostics.weeklyAt")
        .await?
        .and_then(|value| value.parse().ok());
    let now = reverb_core::queue::repo::now();
    if !diagnostics::weekly_due(last, now) {
        return Ok(());
    }
    // Record attempts too, so an offline machine doesn't retry every polling tick.
    state
        .db
        .kv_set("diagnostics.weeklyAt", &now.to_string())
        .await?;
    let mut report = run(app).await;
    if !report.as_ref().is_ok_and(|report| !report.has_errors()) {
        if let Err(error) = state.tools.install_missing().await {
            tracing::warn!(kind = error.kind(), "weekly tools repair failed");
        }
        if let Err(error) = state.tools.update(Tool::Ytdlp).await {
            tracing::warn!(kind = error.kind(), "weekly yt-dlp repair failed");
        }
        report = run(app).await;
        if !report.as_ref().is_ok_and(|report| !report.has_errors()) {
            crate::notifications::show(
                app,
                texts::text(state.settings.get().language, "diagnostics"),
            );
        }
    }
    report.map(|_| ())
}

pub fn initialize(app: &AppHandle) {
    let app = app.clone();
    let cancel = app.state::<AppState>().background_cancel.clone();
    tauri::async_runtime::spawn(async move {
        tokio::select! { _ = cancel.cancelled() => return, _ = tokio::time::sleep(Duration::from_secs(120)) => {} }
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                result = weekly(&app) => { if let Err(error) = result { tracing::warn!(kind = error.kind(), "weekly diagnostics failed"); } }
            }
            let tools = app.state::<AppState>().tools.clone();
            tokio::select! { _ = cancel.cancelled() => break, _ = async { let _guard = GATE.lock().await; tools.background_startup().await; } => {} }
            tokio::select! { _ = cancel.cancelled() => break, _ = tokio::time::sleep(Duration::from_secs(900)) => {} }
        }
    });
}

//! Deep links are received in Rust after the persistent queue is ready.

use reverb_core::integration::{parse_deep_link, DeepLinkAction};
use reverb_core::queue::{EnqueueRequest, PlaylistCtx};
use reverb_core::urlkind::{classify, UrlKind};
use reverb_core::ytdlp::Analysis;
use reverb_core::{CoreError, CoreResult};
use tauri::{AppHandle, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

use crate::state::AppState;

pub fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        for result in [window.show(), window.unminimize(), window.set_focus()] {
            if let Err(error) = result {
                tracing::warn!(%error, "could not focus main window");
            }
        }
    }
}

pub async fn receive(app: &AppHandle, input: &str) -> CoreResult<()> {
    let action = parse_deep_link(input)?;
    show_window(app);
    let state = app.state::<AppState>();
    if let DeepLinkAction::Add { url, profile_id } = action {
        match classify(&url) {
            UrlKind::Video { source_id, .. } => {
                state
                    .queue
                    .enqueue(EnqueueRequest {
                        url,
                        source_id: Some(source_id),
                        profile_id,
                        ..EnqueueRequest::default()
                    })
                    .await?;
            }
            UrlKind::Collection { .. } => {
                let info = state
                    .backend
                    .analyze(&url, &state.background_cancel)
                    .await
                    .map_err(|error| CoreError::coded("analyze", error.to_string()))?;
                let Analysis::Collection { info } = info else {
                    return Err(CoreError::invalid("expected a collection"));
                };
                let mut seen = std::collections::HashSet::new();
                for (position, entry) in info.entries.iter().enumerate() {
                    if !seen.insert(&entry.id)
                        || matches!(
                            entry.title.as_deref(),
                            Some("[Private video]" | "[Deleted video]")
                        )
                    {
                        continue;
                    }
                    state
                        .queue
                        .enqueue(EnqueueRequest {
                            url: entry.url.clone().unwrap_or_else(|| {
                                format!("https://www.youtube.com/watch?v={}", entry.id)
                            }),
                            source_id: Some(entry.id.clone()),
                            title: entry.title.clone(),
                            duration_s: entry.duration,
                            profile_id: profile_id.clone(),
                            playlist_ctx: Some(PlaylistCtx {
                                playlist_title: info.title.clone().unwrap_or_default(),
                                playlist_id: info.id.clone().unwrap_or_default(),
                                index: position as u32 + 1,
                                sync_id: None,
                            }),
                            ..EnqueueRequest::default()
                        })
                        .await?;
                }
            }
            _ => return Err(CoreError::invalid("unsupported deep link URL")),
        }
        state.sink.emit(
            "notice",
            serde_json::json!({
                "level": "success", "i18nKey": "integration.linkAdded"
            }),
        );
    }
    Ok(())
}

fn dispatch(app: AppHandle, input: String) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = receive(&app, &input).await {
            // Never log untrusted arguments, which may contain secrets.
            tracing::warn!(kind = error.kind(), "deep link rejected");
            if let Some(state) = app.try_state::<AppState>() {
                state.sink.emit(
                    "notice",
                    serde_json::json!({
                        "level": "error", "i18nKey": "integration.linkFailed"
                    }),
                );
            }
        }
    });
}

pub fn initialize(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(any(target_os = "linux", all(windows, debug_assertions)))]
    app.deep_link().register_all()?;
    let handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        for url in event.urls() {
            dispatch(handle.clone(), url.to_string());
        }
    });
    // get_current includes the startup CLI argument on Windows/Linux. Use arguments
    // only as a fallback, so a cold launch cannot enqueue the same link twice.
    if let Some(urls) = app.deep_link().get_current()? {
        for url in urls {
            dispatch(app.clone(), url.to_string());
        }
    } else {
        for argument in std::env::args()
            .skip(1)
            .filter(|arg| arg.starts_with("reverb:"))
        {
            dispatch(app.clone(), argument);
        }
    }
    Ok(())
}

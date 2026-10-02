use reverb_core::{CoreError, CoreResult};
use std::str::FromStr;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub fn parse(text: &str) -> CoreResult<Option<Shortcut>> {
    reverb_core::settings::validate_shortcut(text)?;
    if text.is_empty() {
        return Ok(None);
    }
    Shortcut::from_str(text)
        .map(Some)
        .map_err(|_| CoreError::invalid_i18n("Invalid accelerator", "integration.shortcutInvalid"))
}

pub fn apply(app: &AppHandle, old: &str, new: &str) -> CoreResult<()> {
    if old == new {
        return Ok(());
    }
    let (previous, next) = (parse(old).ok().flatten(), parse(new)?);
    if previous == next {
        return Ok(());
    }
    if let Some(next) = next {
        app.global_shortcut().register(next).map_err(|_| {
            CoreError::invalid_i18n("Shortcut is unavailable", "integration.shortcutConflict")
        })?;
    }
    if let Some(previous) = previous {
        if app.global_shortcut().is_registered(previous)
            && app.global_shortcut().unregister(previous).is_err()
        {
            if let Some(next) = next {
                let _ = app.global_shortcut().unregister(next);
            }
            return Err(CoreError::invalid_i18n(
                "Could not change shortcut",
                "integration.shortcutConflict",
            ));
        }
    }
    Ok(())
}

pub fn handler(
    app: &AppHandle,
    _shortcut: &Shortcut,
    event: tauri_plugin_global_shortcut::ShortcutEvent,
) {
    if event.state != ShortcutState::Pressed {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if crate::tray::download_clipboard(&app).await.is_err() {
            let state = app.state::<crate::state::AppState>();
            crate::notifications::show(
                &app,
                reverb_core::desktop::texts::text(state.settings.get().language, "noLink"),
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t4_plugin_accelerator_validation() {
        assert!(parse("Ctrl+Shift+D").unwrap().is_some());
        assert!(parse("Ctrl+").is_err());
        assert_eq!(parse("").unwrap(), None);
    }
}

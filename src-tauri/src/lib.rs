mod commands;
mod headless;
mod state;

use std::sync::Arc;

use reverb_core::{logging, Db, EventSink, SettingsService};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

use crate::state::{AppState, TauriSink};

pub fn run() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = headless::handle(&args) {
        std::process::exit(code);
    }

    tauri::Builder::default()
        .setup(|app| {
            // Fonte única do diretório de dados: o valor do core (arquitetura §4 / F00).
            let paths = headless::resolve_data_paths()?;
            std::fs::create_dir_all(&paths.data_dir)?;

            // O guard precisa viver até o fim do app, senão o log não é descarregado.
            let guard = logging::init(&paths.logs_dir())?;
            app.manage(guard);

            if let Ok(tauri_dir) = app.path().app_data_dir() {
                if tauri_dir != paths.data_dir && !paths.portable && cfg!(not(debug_assertions)) {
                    tracing::error!(
                        tauri = %tauri_dir.display(),
                        core = %paths.data_dir.display(),
                        "app_data_dir do Tauri diverge do core; usando o valor do core"
                    );
                }
            }

            let db = Db::open(&paths.db_file())?;
            let sink: Arc<dyn EventSink> = Arc::new(TauriSink::new(app.handle().clone()));
            let settings = tauri::async_runtime::block_on(SettingsService::new(
                db.clone(),
                Arc::clone(&sink),
            ))?;
            app.manage(AppState {
                paths,
                db,
                settings: Arc::new(settings),
                sink,
            });

            WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Reverb")
                .inner_size(1200.0, 800.0)
                .min_inner_size(900.0, 600.0)
                .decorations(false)
                .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::settings::settings_get,
            commands::settings::settings_update,
            commands::settings::settings_reset,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar o Reverb");
}

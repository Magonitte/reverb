mod commands;
mod headless;

use reverb_core::DataPaths;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

pub fn run() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = headless::handle(&args) {
        std::process::exit(code);
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .setup(|app| {
            // Fonte única do diretório de dados: o valor do core (arquitetura §4 / F00).
            let paths = headless::resolve_data_paths()?;
            if let Ok(tauri_dir) = app.path().app_data_dir() {
                if tauri_dir != paths.data_dir && !paths.portable && cfg!(not(debug_assertions)) {
                    tracing::error!(
                        tauri = %tauri_dir.display(),
                        core = %paths.data_dir.display(),
                        "app_data_dir do Tauri diverge do core; usando o valor do core"
                    );
                }
            }
            std::fs::create_dir_all(&paths.data_dir)?;
            app.manage::<DataPaths>(paths);

            WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Reverb")
                .inner_size(1200.0, 800.0)
                .min_inner_size(900.0, 600.0)
                .decorations(false)
                .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::app_info])
        .run(tauri::generate_context!())
        .expect("erro ao executar o Reverb");
}

mod commands;
mod headless;
mod state;
mod updater;

use std::sync::Arc;

use reverb_core::backend::{DownloadBackend, ToolsContext, YtDlpProcessBackend};
use reverb_core::metadata::cache::system_clock;
use reverb_core::metadata::{Endpoints, MetadataService};
use reverb_core::queue::{HealCoordinator, QueueDeps, QueueService, ToolsHeal, ToolsPipeline};
use reverb_core::ytdlp::YtDlpRunner;
use reverb_core::{logging, Db, EventSink, SettingsService, ToolsConfig, ToolsManager};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

use crate::state::{AppState, TauriSink};

pub fn run() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let headless_update = match headless::classify(&args) {
        Ok(mode) => mode,
        Err(code) => std::process::exit(code),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(move |app| {
            if let Some(mode) = &headless_update {
                let code =
                    tauri::async_runtime::block_on(updater::run_headless(app.handle(), mode));
                std::process::exit(code);
            }
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
            let settings = Arc::new(tauri::async_runtime::block_on(SettingsService::new(
                db.clone(),
                Arc::clone(&sink),
            ))?);
            let tools = Arc::new(ToolsManager::new(
                ToolsConfig::new(headless::resolve_tools_dir(&paths)),
                db.clone(),
                Arc::clone(&settings),
                Arc::clone(&sink),
            )?);
            // Ferramentas faltantes e atualizações automáticas, em segundo plano (§16).
            tauri::async_runtime::spawn(Arc::clone(&tools).background_startup());
            let backend: Arc<dyn DownloadBackend> = Arc::new(YtDlpProcessBackend::new(
                YtDlpRunner::new(Some(Arc::clone(&tools))),
                Arc::new(ToolsContext::new(Arc::clone(&tools), Arc::clone(&settings))),
            ));
            // Um só serviço por processo: os limitadores de taxa dos provedores são dele.
            let metadata = Arc::new(MetadataService::new(
                Arc::clone(&backend),
                Arc::clone(&settings),
                db.clone(),
                &Endpoints::default(),
                system_clock(),
            ));
            let heal = Arc::new(HealCoordinator::new(
                Arc::new(ToolsHeal::new(Arc::clone(&tools), Arc::clone(&settings))),
                db.clone(),
                Arc::clone(&sink),
            ));
            // A fila restaura os jobs do banco (`running` ⇒ `queued`) e começa a processar (§10).
            let queue = tauri::async_runtime::block_on(QueueService::start(QueueDeps {
                db: db.clone(),
                settings: Arc::clone(&settings),
                sink: Arc::clone(&sink),
                runner: Arc::new(
                    ToolsPipeline::new(
                        Arc::clone(&backend),
                        Arc::clone(&tools),
                        paths.data_dir.clone(),
                    )
                    .with_metadata(Arc::clone(&metadata)),
                ),
                heal,
                data_dir: paths.data_dir.clone(),
                start_paused: false,
            }))?;
            let watch_tools = Arc::clone(&tools);
            let background_cancel = tokio_util::sync::CancellationToken::new();
            let syncs = reverb_core::sync::SyncService::new(
                db.clone(),
                settings.clone(),
                backend.clone(),
                queue.clone(),
                sink.clone(),
            );
            // Core background services require the Tauri Tokio runtime during startup.
            tauri::async_runtime::block_on(async {
                syncs.start(background_cancel.clone());
            });
            tauri::async_runtime::spawn(reverb_core::library::watch::run(
                db.clone(),
                Arc::clone(&settings),
                Arc::clone(&sink),
                Arc::new(move || watch_tools.resolve_ffprobe()),
                background_cancel.clone(),
            ));
            app.manage(AppState {
                paths,
                db,
                settings,
                tools,
                queue,
                syncs,
                background_cancel,
                backend,
                metadata,
                sink,
            });
            updater::spawn_auto_check(app.handle().clone());

            // Janela por plataforma (design §1): Windows transparente com Mica; Linux opaca
            // (WebKitGTK é lento com transparência; a UI usa `data-transparency="reduced"`).
            let builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Reverb")
                .inner_size(1200.0, 800.0)
                .min_inner_size(900.0, 600.0)
                .decorations(false);
            #[cfg(windows)]
            let builder = builder.transparent(true).effects(
                tauri::window::EffectsBuilder::new()
                    .effect(tauri::window::Effect::Mica)
                    .build(),
            );
            #[cfg(not(windows))]
            let builder = builder.transparent(false);
            builder.build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::settings::settings_get,
            commands::settings::settings_update,
            commands::settings::settings_reset,
            commands::tools::tools_status,
            commands::tools::tools_install_missing,
            commands::tools::tools_check_updates,
            commands::tools::tools_update,
            commands::tools::tools_rollback,
            commands::media::url_classify,
            commands::media::analyze,
            commands::media::search,
            commands::media::pick_folder,
            commands::media::open_output_dir,
            commands::media::clipboard_read_text,
            commands::media::library_reveal,
            commands::media::library_open_file,
            commands::postprocess::library_cover,
            commands::postprocess::template_preview,
            commands::library::library_list,
            commands::library::library_import,
            commands::library::library_rescan,
            commands::library::tags_read,
            commands::library::tags_write,
            commands::library::pick_audio_file,
            commands::library::pick_image_file,
            commands::library::artwork_read,
            commands::library::artwork_fetch,
            commands::library::library_get,
            commands::library::library_artists,
            commands::library::library_albums,
            commands::library::library_delete,
            commands::library::library_clear,
            commands::library::review_dismiss,
            commands::library::review_list,
            commands::library::review_apply,
            commands::metadata::find_official_version,
            commands::metadata::metadata_preview,
            commands::metadata::metadata_search,
            commands::sync::syncs_list,
            commands::sync::sync_create,
            commands::sync::sync_update,
            commands::sync::sync_delete,
            commands::sync::sync_run,
            commands::sync::sync_items,
            commands::queue::enqueue,
            commands::queue::check_duplicates,
            commands::queue::jobs_list,
            commands::queue::job_cancel,
            commands::queue::job_retry,
            commands::queue::job_remove,
            commands::queue::job_move,
            commands::queue::jobs_clear_finished,
            commands::queue::queue_pause,
            commands::queue::queue_resume,
            commands::queue::queue_state,
            commands::queue::jobs_cancel_all,
            commands::updater::updater_check,
            commands::updater::updater_install,
            commands::updater::app_restart,
        ])
        .build(tauri::generate_context!())
        .expect("erro ao construir o Reverb")
        .run(|handle, event| {
            // Encerra o servidor de PO token (e sua árvore de processos) ao fechar o app.
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = handle.try_state::<AppState>() {
                    tauri::async_runtime::block_on(async {
                        state.background_cancel.cancel();
                        state.syncs.stop();
                        // Jobs em execução voltam para a fila; depois o servidor de PO token cai.
                        state.queue.shutdown().await;
                        state.tools.shutdown().await;
                    });
                }
            }
        });
}

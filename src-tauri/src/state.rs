use std::sync::Arc;

use reverb_core::backend::DownloadBackend;
use reverb_core::metadata::MetadataService;
use reverb_core::queue::QueueService;
use reverb_core::{DataPaths, Db, EventSink, SettingsService, ToolsManager};
use tauri::{AppHandle, Emitter};

/// Estado gerenciado do app, criado no `setup()` (arquitetura §3).
pub struct AppState {
    #[allow(dead_code)] // F02+: ferramentas, fila e biblioteca usam os diretórios.
    pub paths: DataPaths,
    #[allow(dead_code)] // F09+: a biblioteca usa o banco diretamente.
    pub db: Db,
    pub settings: Arc<SettingsService>,
    pub tools: Arc<ToolsManager>,
    pub tools_startup: tokio::sync::Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    pub queue: QueueService,
    pub syncs: Arc<reverb_core::sync::SyncService>,
    pub imports: Arc<reverb_core::import::ImportService>,
    pub artists: Arc<reverb_core::artists::ArtistService>,
    pub background_cancel: tokio_util::sync::CancellationToken,
    /// Análise e busca (F07) usam o mesmo backend dos downloads.
    pub backend: Arc<dyn DownloadBackend>,
    /// Versão oficial, pré-visualização e busca de metadados (F08); o mesmo serviço do pipeline.
    pub metadata: Arc<MetadataService>,
    #[allow(dead_code)] // F02+: ferramentas e fila emitem eventos pelo sink.
    pub sink: Arc<dyn EventSink>,
}

/// `EventSink` que publica os eventos do core na WebView com `app.emit`.
pub struct TauriSink {
    handle: AppHandle,
}

impl TauriSink {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }
}

impl EventSink for TauriSink {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        if let Err(e) = self.handle.emit(event, payload) {
            tracing::warn!(event, error = %e, "falha ao emitir evento");
        }
    }
}

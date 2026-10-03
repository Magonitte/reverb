//! Quem executa o pipeline de um job. A fila só conhece este trait; o app usa `ToolsPipeline`
//! (resolve o FFmpeg a cada job) e os testes usam um `DownloadPipeline` com `FakeBackend`.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::backend::DownloadBackend;
use crate::metadata::MetadataService;
use crate::pipeline::postprocess::PostProcessor;
use crate::pipeline::{DownloadPipeline, PipelineEvent, PipelineJob, PipelineOutput};
use crate::tools::{Tool, ToolsManager};
use crate::ytdlp::errors::{DownloadError, ErrorKind};

pub type EventFn<'a> = &'a (dyn Fn(PipelineEvent) + Send + Sync);

#[async_trait]
pub trait PipelineRunner: Send + Sync {
    async fn run(
        &self,
        job: &PipelineJob,
        cancel: &CancellationToken,
        on_event: EventFn<'_>,
    ) -> Result<PipelineOutput, DownloadError>;
}

#[async_trait]
impl PipelineRunner for DownloadPipeline {
    async fn run(
        &self,
        job: &PipelineJob,
        cancel: &CancellationToken,
        on_event: EventFn<'_>,
    ) -> Result<PipelineOutput, DownloadError> {
        DownloadPipeline::run(self, job, cancel, on_event).await
    }
}

/// Pipeline real: o FFmpeg instalado pode mudar (atualização), então a pasta é resolvida a cada job.
pub struct ToolsPipeline {
    db: Option<crate::Db>,
    backend: Arc<dyn DownloadBackend>,
    tools: Arc<ToolsManager>,
    data_dir: PathBuf,
    metadata: Option<Arc<MetadataService>>,
    postprocess: Arc<PostProcessor>,
}

impl ToolsPipeline {
    pub fn new(
        backend: Arc<dyn DownloadBackend>,
        tools: Arc<ToolsManager>,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            db: None,
            backend,
            tools,
            data_dir,
            metadata: None,
            postprocess: Arc::new(PostProcessor::default()),
        }
    }

    pub fn with_database(mut self, db: crate::Db) -> Self {
        self.db = Some(db);
        self
    }

    /// Liga os passos de metadados (F08).
    pub fn with_metadata(mut self, metadata: Arc<MetadataService>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

#[async_trait]
impl PipelineRunner for ToolsPipeline {
    async fn run(
        &self,
        job: &PipelineJob,
        cancel: &CancellationToken,
        on_event: EventFn<'_>,
    ) -> Result<PipelineOutput, DownloadError> {
        let ffmpeg = self
            .tools
            .resolve(Tool::Ffmpeg)
            .map_err(|e| DownloadError::new(ErrorKind::Ffmpeg, e.to_string()))?;
        let ffmpeg_dir = ffmpeg
            .parent()
            .map(PathBuf::from)
            .ok_or_else(|| DownloadError::new(ErrorKind::Ffmpeg, "FFmpeg sem pasta"))?;
        if job.options.upgrade_library_id.is_some() {
            let db = self.db.as_ref().ok_or_else(|| {
                DownloadError::new(ErrorKind::Unknown, "Upgrade database missing")
            })?;
            let _guard = self.tools.acquire_run().await;
            return crate::quality::upgrade::run(
                db,
                self.backend.as_ref(),
                &self.data_dir,
                &ffmpeg_dir,
                job,
                cancel,
                on_event,
            )
            .await;
        }
        DownloadPipeline::new(
            Arc::clone(&self.backend),
            self.data_dir.clone(),
            ffmpeg_dir,
            Some(Arc::clone(&self.tools)),
        )
        .with_metadata(self.metadata.clone())
        .with_postprocessing(Arc::clone(&self.postprocess))
        .run(job, cancel, on_event)
        .await
    }
}

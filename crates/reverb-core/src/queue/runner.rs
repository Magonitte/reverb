//! Quem executa o pipeline de um job. A fila só conhece este trait; o app usa `ToolsPipeline`
//! (resolve o FFmpeg a cada job) e os testes usam um `DownloadPipeline` com `FakeBackend`.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::backend::DownloadBackend;
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
    backend: Arc<dyn DownloadBackend>,
    tools: Arc<ToolsManager>,
    data_dir: PathBuf,
}

impl ToolsPipeline {
    pub fn new(
        backend: Arc<dyn DownloadBackend>,
        tools: Arc<ToolsManager>,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            backend,
            tools,
            data_dir,
        }
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
        DownloadPipeline::new(
            Arc::clone(&self.backend),
            self.data_dir.clone(),
            ffmpeg_dir,
            Some(Arc::clone(&self.tools)),
        )
        .run(job, cancel, on_event)
        .await
    }
}

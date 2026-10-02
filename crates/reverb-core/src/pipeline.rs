//! Um job de ponta a ponta: resolve a fonte ⇒ baixa ⇒ converte (se o perfil pedir) ⇒ identifica
//! os metadados ⇒ move para `<destino>/<título sanitizado>.<ext>`. Os passos de metadados só
//! existem quando há um `MetadataService` (F08). A pasta temporária do job é apagada ao final,
//! em qualquer resultado (sucesso, erro ou cancelamento).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::backend::{DownloadBackend, JobSpec};
use crate::metadata::{IdentifyInput, MetadataResult, MetadataService, SourcePlan};
use crate::organize::{sanitize_path, unique_path};
use crate::profiles::{Profile, ResolvedProfile};
use crate::tools::ToolsManager;
use crate::transcode;
use crate::workspace::JobWorkspace;
use crate::ytdlp::errors::{DownloadError, ErrorKind};
use crate::ytdlp::{DoneInfo, ProgressUpdate};

#[derive(Debug, Clone)]
pub struct PipelineJob {
    /// Nome da pasta `tmp/<job_id>/`.
    pub job_id: String,
    pub url: String,
    pub profile: &'static Profile,
    pub out_dir: PathBuf,
    pub sponsorblock: Option<Vec<String>>,
    /// Edição do usuário (Preview): prioridade máxima, pula a identificação e a troca de fonte.
    pub metadata_override: Option<serde_json::Value>,
    /// Sobrescreve `fetchMetadata` só para este job.
    pub fetch_metadata: Option<bool>,
}

#[derive(Debug, Clone)]
pub enum PipelineEvent {
    Download(ProgressUpdate),
    /// Progresso da conversão (0–100).
    Convert(u8),
    /// Começou a analisar o vídeo e a escolher a fonte (`resolve_source`).
    Analyzing,
    /// A fonte foi trocada pela versão oficial.
    SourceSwitched {
        url: String,
        source_id: String,
    },
    /// Começou a identificar os metadados (`identify`).
    Identifying,
    Identified(Box<MetadataResult>),
}

#[derive(Debug, Clone)]
pub struct PipelineOutput {
    pub path: PathBuf,
    pub done: DoneInfo,
    pub profile: ResolvedProfile,
    /// Resultado da identificação (só com `MetadataService`).
    pub metadata: Option<MetadataResult>,
}

pub struct DownloadPipeline {
    backend: Arc<dyn DownloadBackend>,
    data_dir: PathBuf,
    ffmpeg_dir: PathBuf,
    tools: Option<Arc<ToolsManager>>,
    metadata: Option<Arc<MetadataService>>,
}

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn io_error(context: &str, error: std::io::Error) -> DownloadError {
    DownloadError::new(ErrorKind::Disk, format!("{context}: {error}"))
}

/// Move com `rename`; se o destino estiver em outro volume, copia e apaga.
async fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if tokio::fs::rename(from, to).await.is_ok() {
        return Ok(());
    }
    tokio::fs::copy(from, to).await?;
    tokio::fs::remove_file(from).await
}

impl DownloadPipeline {
    pub fn new(
        backend: Arc<dyn DownloadBackend>,
        data_dir: PathBuf,
        ffmpeg_dir: PathBuf,
        tools: Option<Arc<ToolsManager>>,
    ) -> Self {
        Self {
            backend,
            data_dir,
            ffmpeg_dir,
            tools,
            metadata: None,
        }
    }

    /// Liga os passos `resolve_source` e `identify` (F08).
    pub fn with_metadata(mut self, metadata: Option<Arc<MetadataService>>) -> Self {
        self.metadata = metadata;
        self
    }

    pub async fn run(
        &self,
        job: &PipelineJob,
        cancel: &CancellationToken,
        on_event: &(dyn Fn(PipelineEvent) + Send + Sync),
    ) -> Result<PipelineOutput, DownloadError> {
        let workspace = JobWorkspace::new(&self.data_dir, &job.job_id)
            .map_err(|e| DownloadError::new(ErrorKind::Disk, e.to_string()))?;
        let plan = self.resolve_source(job, cancel, on_event).await?;
        let url = plan
            .as_ref()
            .map_or_else(|| job.url.clone(), |p| p.url.clone());
        let spec = JobSpec {
            url,
            tmp_dir: workspace.path().to_path_buf(),
            sponsorblock: job.sponsorblock.clone(),
        };
        let on_progress = |update| on_event(PipelineEvent::Download(update));
        let done = self.backend.download(&spec, &on_progress, cancel).await?;

        let downloaded = PathBuf::from(&done.filepath);
        if !downloaded.is_file() {
            return Err(DownloadError::new(
                ErrorKind::Unknown,
                format!("arquivo baixado não encontrado: {}", downloaded.display()),
            ));
        }

        let profile = job.profile.resolve(&done.ext);
        if profile.fallback {
            tracing::warn!(ext = %done.ext, "formato de origem incomum: convertendo para Opus 160k");
        }
        let ready = if profile.needs_conversion() {
            self.convert(&downloaded, &profile, &done, &workspace, cancel, on_event)
                .await?
        } else {
            downloaded
        };

        let metadata = self
            .identify(job, plan.as_ref(), &ready, &done, cancel, on_event)
            .await?;

        tokio::fs::create_dir_all(&job.out_dir)
            .await
            .map_err(|e| io_error("não foi possível criar a pasta de destino", e))?;
        let target = unique_path(&sanitize_path(&job.out_dir, &[&done.title], &profile.ext));
        move_file(&ready, &target)
            .await
            .map_err(|e| io_error("não foi possível mover o arquivo para o destino", e))?;
        Ok(PipelineOutput {
            path: target,
            done,
            profile,
            metadata,
        })
    }

    /// `resolve_source`: analisa o vídeo e decide a fonte final (pode trocar pela versão oficial).
    async fn resolve_source(
        &self,
        job: &PipelineJob,
        cancel: &CancellationToken,
        on_event: &(dyn Fn(PipelineEvent) + Send + Sync),
    ) -> Result<Option<SourcePlan>, DownloadError> {
        let Some(metadata) = &self.metadata else {
            return Ok(None);
        };
        on_event(PipelineEvent::Analyzing);
        let plan = metadata
            .resolve_source(&job.url, job.metadata_override.is_some(), cancel)
            .await?;
        if let (true, Some(video)) = (plan.switched(), plan.video.as_ref()) {
            on_event(PipelineEvent::SourceSwitched {
                url: plan.url.clone(),
                source_id: video.id.clone(),
            });
        }
        Ok(Some(plan))
    }

    /// `identify`: metadados com a duração real do arquivo (ffprobe).
    async fn identify(
        &self,
        job: &PipelineJob,
        plan: Option<&SourcePlan>,
        file: &Path,
        done: &DoneInfo,
        cancel: &CancellationToken,
        on_event: &(dyn Fn(PipelineEvent) + Send + Sync),
    ) -> Result<Option<MetadataResult>, DownloadError> {
        let (Some(metadata), Some(plan)) = (&self.metadata, plan) else {
            return Ok(None);
        };
        on_event(PipelineEvent::Identifying);
        let ffprobe = self.ffmpeg_dir.join(exe("ffprobe"));
        let duration = match transcode::probe(&ffprobe, file).await {
            Ok(info) if info.duration_s > 0.0 => Some(info.duration_s),
            _ => done.duration,
        };
        let identified = metadata.identify(IdentifyInput {
            plan,
            fallback_title: &done.title,
            duration_s: duration,
            user_override: job.metadata_override.as_ref(),
            fetch_metadata: job.fetch_metadata,
        });
        let result = tokio::select! {
            result = identified => result,
            () = cancel.cancelled() => return Err(DownloadError::cancelled()),
        };
        on_event(PipelineEvent::Identified(Box::new(result.clone())));
        Ok(Some(result))
    }

    async fn convert(
        &self,
        downloaded: &Path,
        profile: &ResolvedProfile,
        done: &DoneInfo,
        workspace: &JobWorkspace,
        cancel: &CancellationToken,
        on_event: &(dyn Fn(PipelineEvent) + Send + Sync),
    ) -> Result<PathBuf, DownloadError> {
        let _guard = match &self.tools {
            Some(tools) => Some(tools.acquire_run().await),
            None => None,
        };
        let ffmpeg = self.ffmpeg_dir.join(exe("ffmpeg"));
        let ffprobe = self.ffmpeg_dir.join(exe("ffprobe"));
        let duration = match transcode::probe(&ffprobe, downloaded).await {
            Ok(info) => info.duration_s,
            Err(_) if done.duration.is_some_and(|d| d > 0.0) => done.duration.unwrap_or(0.0),
            Err(error) => return Err(error),
        };
        let output = workspace.path().join(format!("converted.{}", profile.ext));
        let on_percent = |percent| on_event(PipelineEvent::Convert(percent));
        transcode::convert(
            &ffmpeg,
            downloaded,
            &output,
            profile,
            duration,
            cancel,
            &on_percent,
        )
        .await?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests;

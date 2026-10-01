//! Um job de ponta a ponta: baixa ⇒ converte (se o perfil pedir) ⇒ move para
//! `<destino>/<título sanitizado>.<ext>`. A pasta temporária do job é apagada ao final, em
//! qualquer resultado (sucesso, erro ou cancelamento).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::backend::{DownloadBackend, JobSpec};
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
}

#[derive(Debug, Clone)]
pub enum PipelineEvent {
    Download(ProgressUpdate),
    /// Progresso da conversão (0–100).
    Convert(u8),
}

#[derive(Debug, Clone)]
pub struct PipelineOutput {
    pub path: PathBuf,
    pub done: DoneInfo,
    pub profile: ResolvedProfile,
}

pub struct DownloadPipeline {
    backend: Arc<dyn DownloadBackend>,
    data_dir: PathBuf,
    ffmpeg_dir: PathBuf,
    tools: Option<Arc<ToolsManager>>,
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
        }
    }

    pub async fn run(
        &self,
        job: &PipelineJob,
        cancel: &CancellationToken,
        on_event: &(dyn Fn(PipelineEvent) + Send + Sync),
    ) -> Result<PipelineOutput, DownloadError> {
        let workspace = JobWorkspace::new(&self.data_dir, &job.job_id)
            .map_err(|e| DownloadError::new(ErrorKind::Disk, e.to_string()))?;
        let spec = JobSpec {
            url: job.url.clone(),
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
        })
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

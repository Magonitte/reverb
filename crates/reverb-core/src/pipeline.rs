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
use crate::queue::{JobOptions, JobStage, PlaylistCtx};
use crate::tools::ToolsManager;
use crate::transcode;
use crate::workspace::JobWorkspace;
use crate::ytdlp::errors::{DownloadError, ErrorKind};
use crate::ytdlp::{DoneInfo, ProgressUpdate};
use crate::Settings;

pub mod postprocess;
use postprocess::{PostProcessor, Publication};

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
    pub settings: Option<Settings>,
    pub options: JobOptions,
    pub playlist_ctx: Option<PlaylistCtx>,
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
    Stage(JobStage),
    Warning(String),
}

#[derive(Debug)]
pub struct PipelineOutput {
    pub path: PathBuf,
    pub done: DoneInfo,
    pub profile: ResolvedProfile,
    /// Resultado da identificação (só com `MetadataService`).
    pub metadata: Option<MetadataResult>,
    pub library_file: Option<crate::library::DownloadedFile>,
    pub publication: Option<Publication>,
    pub replacement: Option<crate::quality::replace::Replacement>,
    pub upgrade_id: Option<i64>,
    pub chapter_files: Vec<crate::library::DownloadedFile>,
    pub chapter_publications: Vec<Publication>,
}

pub struct DownloadPipeline {
    backend: Arc<dyn DownloadBackend>,
    data_dir: PathBuf,
    ffmpeg_dir: PathBuf,
    tools: Option<Arc<ToolsManager>>,
    metadata: Option<Arc<MetadataService>>,
    postprocess: Option<Arc<PostProcessor>>,
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
            postprocess: None,
        }
    }

    /// Liga os passos `resolve_source` e `identify` (F08).
    pub fn with_metadata(mut self, metadata: Option<Arc<MetadataService>>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_postprocessing(mut self, processor: Arc<PostProcessor>) -> Self {
        self.postprocess = Some(processor);
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
        if crate::sources::provider_id(&job.url) == "soundcloud"
            && done.format_id.as_deref() != Some("download")
        {
            on_event(PipelineEvent::Warning(
                "warnings.originalUnavailable".into(),
            ));
        }
        if profile.fallback {
            tracing::warn!(ext = %done.ext, "formato de origem incomum: convertendo para Opus 160k");
        }
        let ready = if profile.needs_conversion() {
            self.convert(&downloaded, &profile, &done, &workspace, cancel, on_event)
                .await?
        } else {
            downloaded
        };

        let splitting = plan
            .as_ref()
            .and_then(|p| p.video.as_ref())
            .is_some_and(|v| {
                crate::quality::chapters::should_split(
                    v,
                    job.settings
                        .as_ref()
                        .map_or(crate::settings::SplitChapters::Ask, |s| s.split_chapters),
                    job.options.split_chapters,
                )
            });
        let ready = if !splitting && job.settings.as_ref().is_some_and(|s| s.trim_silence) {
            let _guard = match &self.tools {
                Some(tools) => Some(tools.acquire_run().await),
                None => None,
            };
            let output = workspace.path().join(format!("trimmed.{}", profile.ext));
            crate::quality::audio::silence(
                &self.ffmpeg_dir.join(exe("ffmpeg")),
                &ready,
                &output,
                &profile,
                done.abr.unwrap_or(160.0),
                cancel,
            )
            .await?;
            if job.profile.id == "original" {
                on_event(PipelineEvent::Warning("warnings.trimReencode".into()));
            }
            output
        } else {
            ready
        };

        let mut metadata = self
            .identify(job, plan.as_ref(), &ready, &done, cancel, on_event)
            .await?;

        if let (Some(result), Some(settings), Some(tools)) =
            (&mut metadata, &job.settings, &self.tools)
        {
            if result.confidence < settings.confidence_auto_apply
                && !settings.acoustid_key.is_empty()
                && !settings.offline_mode
                && job.metadata_override.is_none()
            {
                let lookup = async {
                    if tools.resolve(crate::tools::Tool::Fpcalc).is_err() {
                        tools.update(crate::tools::Tool::Fpcalc).await?;
                    }
                    let _guard = tools.acquire_run().await;
                    let fpcalc = tools.resolve(crate::tools::Tool::Fpcalc)?;
                    crate::quality::acoustid::lookup(
                        &fpcalc,
                        &ready,
                        &settings.acoustid_key,
                        "https://api.acoustid.org/v2/lookup",
                    )
                    .await
                };
                let identified = tokio::select! {
                    _ = cancel.cancelled() => return Err(DownloadError::cancelled()),
                    result = lookup => result,
                };
                match identified {
                    Ok(candidates) => {
                        if let Some(best) =
                            candidates.first().filter(|c| c.score > result.confidence)
                        {
                            result.confidence = best.score;
                            result.source = "acoustid".into();
                            result.bucket = if best.score >= settings.confidence_auto_apply {
                                result.fields.apply_candidate(&best.candidate);
                                crate::metadata::Bucket::Auto
                            } else {
                                crate::metadata::Bucket::Review
                            };
                            result.fields.mb_recording_id = best.candidate.mb_recording_id.clone();
                            result.candidates = candidates;
                            on_event(PipelineEvent::Identified(Box::new(result.clone())));
                        }
                    }
                    Err(_) => on_event(PipelineEvent::Warning("warnings.fingerprint".into())),
                }
            }
        }
        if let Some(processor) = &self.postprocess {
            // Mantém probe/loudness protegidos contra a troca de versão das ferramentas.
            let _guard = match &self.tools {
                Some(tools) => Some(tools.acquire_run().await),
                None => None,
            };
            if let Some(video) = plan.as_ref().and_then(|p| p.video.as_ref()).filter(|v| {
                crate::quality::chapters::should_split(
                    v,
                    job.settings
                        .as_ref()
                        .map_or(crate::settings::SplitChapters::Ask, |s| s.split_chapters),
                    job.options.split_chapters,
                )
            }) {
                let mut files = Vec::new();
                let mut publications = Vec::new();
                for (index, chapter) in video.chapters.iter().enumerate() {
                    let segment = workspace
                        .path()
                        .join(format!("chapter-{index}.{}", profile.ext));
                    crate::quality::audio::segment(
                        &self.ffmpeg_dir.join(exe("ffmpeg")),
                        &ready,
                        &segment,
                        chapter.start_time,
                        chapter.end_time,
                        &crate::profiles::profile("original")
                            .expect("original profile")
                            .resolve(&profile.ext),
                        cancel,
                    )
                    .await?;
                    let segment = if job.settings.as_ref().is_some_and(|s| s.trim_silence) {
                        let output = workspace
                            .path()
                            .join(format!("chapter-{index}-trimmed.{}", profile.ext));
                        crate::quality::audio::silence(
                            &self.ffmpeg_dir.join(exe("ffmpeg")),
                            &segment,
                            &output,
                            &profile,
                            done.abr.unwrap_or(160.0),
                            cancel,
                        )
                        .await?;
                        if job.profile.id == "original" {
                            on_event(PipelineEvent::Warning("warnings.trimReencode".into()));
                        }
                        output
                    } else {
                        segment
                    };
                    let mut part_job = job.clone();
                    part_job.options.fetch_lyrics = Some(false);
                    let mut part_meta =
                        metadata
                            .clone()
                            .unwrap_or_else(|| crate::metadata::MetadataResult {
                                fields: crate::metadata::MetadataFields::default(),
                                confidence: 1.0,
                                source: "chapters".into(),
                                bucket: crate::metadata::Bucket::Auto,
                                candidates: vec![],
                                content_type: crate::metadata::ContentType::Music,
                                isrc: None,
                                official: None,
                            });
                    part_meta.fields.title = crate::quality::chapters::clean_title(&chapter.title);
                    part_meta.fields.album = Some(
                        crate::metadata::parse_title::parse_title(
                            &video.title,
                            video.channel.as_deref(),
                        )
                        .title,
                    );
                    part_meta.fields.album_artist = part_meta
                        .fields
                        .artist
                        .clone()
                        .or_else(|| video.artist.clone())
                        .or_else(|| video.channel.clone());
                    part_meta.fields.artist = part_meta.fields.album_artist.clone();
                    part_meta.fields.track_no = Some(index as u32 + 1);
                    part_meta.fields.track_total = Some(video.chapters.len() as u32);
                    part_meta.isrc = None;
                    let (file, publication) = processor
                        .run(
                            &part_job,
                            &segment,
                            &profile.ext,
                            &done,
                            Some(&part_meta),
                            plan.as_ref(),
                            &self.ffmpeg_dir,
                            cancel,
                            on_event,
                        )
                        .await?;
                    files.push(file);
                    publications.push(publication);
                }
                let file = files.remove(0);
                let publication = publications.remove(0);
                return Ok(PipelineOutput {
                    path: PathBuf::from(&file.file_path),
                    done,
                    profile,
                    metadata,
                    library_file: Some(file),
                    publication: Some(publication),
                    replacement: None,
                    upgrade_id: None,
                    chapter_files: files,
                    chapter_publications: publications,
                });
            }
            let (file, publication) = processor
                .run(
                    job,
                    &ready,
                    &profile.ext,
                    &done,
                    metadata.as_ref(),
                    plan.as_ref(),
                    &self.ffmpeg_dir,
                    cancel,
                    on_event,
                )
                .await?;
            return Ok(PipelineOutput {
                path: PathBuf::from(&file.file_path),
                done,
                profile,
                metadata,
                library_file: Some(file),
                publication: Some(publication),
                replacement: None,
                upgrade_id: None,
                chapter_files: vec![],
                chapter_publications: vec![],
            });
        }

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
            library_file: None,
            publication: None,
            replacement: None,
            upgrade_id: None,
            chapter_files: vec![],
            chapter_publications: vec![],
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

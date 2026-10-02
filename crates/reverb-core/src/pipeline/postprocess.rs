//! Pós-processamento da fila e publicação reversível até o commit da biblioteca.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use super::{exe, PipelineEvent, PipelineJob};
use crate::artwork::{self, ArtworkClient, CoverCandidate};
use crate::library::DownloadedFile;
use crate::lyrics::{LyricsClient, LyricsQuery};
use crate::metadata::{ContentType, MetadataFields, MetadataResult, SourcePlan};
use crate::organize::template::{render, TemplateContext};
use crate::queue::JobStage;
use crate::tagging::{self, TagCover, TrackTags};
use crate::transcode;
use crate::ytdlp::{DoneInfo, DownloadError, ErrorKind};
use crate::{CoreError, CoreResult};

pub struct PostProcessor {
    artwork: ArtworkClient,
    lyrics: Arc<LyricsClient>,
}

impl Default for PostProcessor {
    fn default() -> Self {
        Self::new(ArtworkClient::default(), Arc::new(LyricsClient::default()))
    }
}

impl PostProcessor {
    pub fn new(artwork: ArtworkClient, lyrics: Arc<LyricsClient>) -> Self {
        Self { artwork, lyrics }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn run(
        &self,
        job: &PipelineJob,
        ready: &Path,
        ext: &str,
        done: &DoneInfo,
        metadata: Option<&MetadataResult>,
        plan: Option<&SourcePlan>,
        ffmpeg_dir: &Path,
        cancel: &CancellationToken,
        on_event: &(dyn Fn(PipelineEvent) + Send + Sync),
    ) -> Result<(DownloadedFile, Publication), DownloadError> {
        let settings = job.settings.clone().unwrap_or_default();
        let content_type = metadata
            .map(|m| m.content_type)
            .or_else(|| plan.map(|p| p.content_type))
            .unwrap_or(ContentType::Other);
        let fields = metadata
            .map(|m| m.fields.clone())
            .unwrap_or_else(|| MetadataFields {
                title: done.title.clone(),
                ..Default::default()
            });
        let source_url = plan.map_or(job.url.as_str(), |p| p.url.as_str());
        let video = plan.and_then(|p| p.video.as_ref());
        let probe = transcode::probe(&ffmpeg_dir.join(exe("ffprobe")), ready).await?;
        let mut tags = TrackTags::from_metadata(&fields, source_url);
        tags.isrc = metadata.and_then(|m| m.isrc.clone());
        let mut cover_source = None;
        let mut synced = None;
        let mut gain_db = None;
        check_cancel(cancel)?;
        if !settings.offline_mode && job.options.fetch_artwork.unwrap_or(settings.fetch_artwork) {
            on_event(PipelineEvent::Stage(JobStage::Artwork));
            let thumbnail = video.and_then(|v| v.thumbnail.as_deref());
            let candidates = metadata
                .map(|m| artwork::candidates(m, thumbnail, settings.confidence_auto_apply))
                .unwrap_or_else(|| {
                    thumbnail
                        .map(|url| {
                            vec![CoverCandidate {
                                url: url.into(),
                                source: "youtube".into(),
                            }]
                        })
                        .unwrap_or_default()
                });
            let found = tokio::select! { result = self.artwork.fetch(&candidates) => result, () = cancel.cancelled() => return Err(DownloadError::cancelled()) };
            if let Some(cover) = found {
                cover_source = Some(cover.cover_source);
                tags.cover = Some(TagCover {
                    mime_type: "image/jpeg".into(),
                    data: cover.jpeg,
                });
            } else {
                on_event(PipelineEvent::Warning("warnings.artwork".into()));
            }
        }
        if content_type == ContentType::Music
            && !settings.offline_mode
            && job.options.fetch_lyrics.unwrap_or(settings.fetch_lyrics)
        {
            on_event(PipelineEvent::Stage(JobStage::Lyrics));
            let query = LyricsQuery {
                artist: tags.artist.clone().unwrap_or_default(),
                title: tags.title.clone(),
                album: tags.album.clone(),
                duration_s: probe.duration_s,
            };
            let found = tokio::select! { result = self.lyrics.fetch(&query) => result, () = cancel.cancelled() => return Err(DownloadError::cancelled()) };
            match found {
                Ok(Some(lyrics)) => {
                    tags.set_lyrics(&lyrics);
                    synced = lyrics.synced;
                }
                Ok(None) | Err(_) => on_event(PipelineEvent::Warning("warnings.lyrics".into())),
            }
        }
        if settings.normalize_volume {
            on_event(PipelineEvent::Stage(JobStage::Loudness));
            match crate::loudness::analyze(&ffmpeg_dir.join(exe("ffmpeg")), ready, cancel)
                .await
                .and_then(|n| n.replay_gain().map_err(disk_error))
            {
                Ok(gain) => {
                    tags.set_replay_gain(&gain, ext == "opus");
                    gain_db = Some(gain.track_gain_db);
                }
                Err(error) if error.kind == ErrorKind::Cancelled => return Err(error),
                Err(_) => on_event(PipelineEvent::Warning("warnings.loudness".into())),
            }
        }
        check_cancel(cancel)?;
        on_event(PipelineEvent::Stage(JobStage::Tagging));
        let (path, write) = (ready.to_owned(), tags.clone());
        blocking(move || tagging::write_tags(&path, &write)).await?;
        check_cancel(cancel)?;
        on_event(PipelineEvent::Stage(JobStage::Moving));
        let ctx = TemplateContext {
            output_dir: job.out_dir.clone(),
            extension: ext.to_owned(),
            language: settings.language,
            content_type,
            auto_organize: job.options.auto_organize.unwrap_or(settings.auto_organize),
            channel: video.and_then(|v| v.channel.clone().or_else(|| v.uploader.clone())),
            source_id: Some(done.id.clone()),
            playlist: job.playlist_ctx.as_ref().map(|p| p.playlist_title.clone()),
            playlist_index: job.playlist_ctx.as_ref().map(|p| p.index),
        };
        let target = render(&settings.file_template, &tags, &ctx).map_err(disk_error)?;
        let ready = ready.to_owned();
        let lrc = settings.write_lrc_file.then_some(synced.clone()).flatten();
        let cover = settings
            .write_folder_cover
            .then(|| tags.cover.as_ref().map(|c| c.data.clone()))
            .flatten();
        let publication =
            blocking(move || publish(&ready, &target, lrc.as_deref(), cover.as_deref())).await?;
        check_cancel(cancel)?; // `publication` limpa tudo ao sair por cancelamento.
        let file = DownloadedFile {
            file_path: publication.paths[0].to_string_lossy().into_owned(),
            tags,
            probe: Some(probe),
            source_abr_kbps: done.abr,
            content_type,
            has_synced_lyrics: synced.is_some(),
            cover_source,
            replaygain_db: gain_db,
        };
        Ok((file, publication))
    }
}

fn check_cancel(cancel: &CancellationToken) -> Result<(), DownloadError> {
    if cancel.is_cancelled() {
        Err(DownloadError::cancelled())
    } else {
        Ok(())
    }
}

fn disk_error(error: impl std::fmt::Display) -> DownloadError {
    DownloadError::new(ErrorKind::Disk, error.to_string())
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> CoreResult<T> + Send + 'static,
) -> Result<T, DownloadError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(disk_error)?
        .map_err(disk_error)
}

#[derive(Debug)]
pub struct Publication {
    paths: Vec<PathBuf>,
    committed: bool,
}

impl Publication {
    pub fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for Publication {
    fn drop(&mut self) {
        if !self.committed {
            for path in self.paths.iter().rev() {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

fn write_sidecar(path: &Path, bytes: &[u8]) -> CoreResult<bool> {
    let mut file = tempfile::Builder::new()
        .prefix(".reverb-sidecar-")
        .tempfile_in(path.parent().unwrap_or_else(|| Path::new(".")))?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    match file.persist_noclobber(path) {
        Ok(_) => Ok(true),
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(CoreError::from(error.error)),
    }
}

fn publish(
    ready: &Path,
    target: &Path,
    lrc: Option<&str>,
    cover: Option<&[u8]>,
) -> CoreResult<Publication> {
    // Uma letra órfã também reserva o nome; nunca acoplar letra antiga ao áudio novo.
    let target = crate::organize::sanitize::unique_path_for(target, |path| {
        std::fs::symlink_metadata(path).is_ok()
            || (lrc.is_some() && std::fs::symlink_metadata(path.with_extension("lrc")).is_ok())
    });
    let audio = crate::organize::move_into_library(ready, &target)?;
    let mut publication = Publication {
        paths: vec![audio.clone()],
        committed: false,
    };
    if let Some(lrc) = lrc {
        let path = audio.with_extension("lrc");
        if !write_sidecar(&path, lrc.as_bytes())? {
            return Err(CoreError::coded("disk", "A letra do destino já existe"));
        }
        publication.paths.push(path);
    }
    if let Some(cover) = cover {
        let path = audio
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("cover.jpg");
        if write_sidecar(&path, cover)? {
            publication.paths.push(path);
        }
    }
    Ok(publication)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publicacao_sem_commit_limpa_audio_letra_e_capa() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.opus");
        std::fs::write(&source, b"audio").unwrap();
        let target = dir.path().join("album/title.opus");
        let publication = publish(&source, &target, Some("[00:01]letra"), Some(b"JPEG")).unwrap();
        assert!(target.is_file() && target.with_extension("lrc").is_file());
        assert!(target.parent().unwrap().join("cover.jpg").is_file());
        drop(publication);
        assert_eq!(
            std::fs::read_dir(target.parent().unwrap()).unwrap().count(),
            0
        );
    }

    #[test]
    fn sidecars_preservam_capa_existente_e_evitam_letra_orfa() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.opus");
        std::fs::write(&source, b"audio").unwrap();
        let target = dir.path().join("title.opus");
        std::fs::write(target.with_extension("lrc"), b"antiga").unwrap();
        std::fs::write(dir.path().join("cover.jpg"), b"capa existente").unwrap();
        let mut publication = publish(&source, &target, Some("nova"), Some(b"nova capa")).unwrap();
        assert_eq!(publication.paths[0], dir.path().join("title (2).opus"));
        publication.commit();
        drop(publication);
        assert_eq!(
            std::fs::read(dir.path().join("cover.jpg")).unwrap(),
            b"capa existente"
        );
        assert_eq!(
            std::fs::read(target.with_extension("lrc")).unwrap(),
            b"antiga"
        );
        assert_eq!(
            std::fs::read(dir.path().join("title (2).lrc")).unwrap(),
            b"nova"
        );
    }

    #[test]
    fn rollback_preserva_diretorio_preexistente_no_nome_da_capa() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.opus");
        std::fs::write(&source, b"audio").unwrap();
        let target = dir.path().join("title.opus");
        std::fs::create_dir(dir.path().join("cover.jpg")).unwrap();
        let publication = publish(&source, &target, Some("letra"), Some(b"JPEG"));
        // Uma entrada existente não deve ser trocada, seja arquivo ou diretório.
        if let Ok(publication) = publication {
            drop(publication);
        }
        assert!(!target.exists() && !target.with_extension("lrc").exists());
        assert!(dir.path().join("cover.jpg").is_dir());
    }
}

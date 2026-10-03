use crate::backend::{DownloadBackend, JobSpec};
use crate::library::{DownloadedFile, LibraryItem};
use crate::pipeline::{PipelineEvent, PipelineJob, PipelineOutput};
use crate::ytdlp::{Analysis, DownloadError, ErrorKind};
use crate::{CoreResult, Db};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpgradeCandidate {
    pub item: LibraryItem,
    pub available_abr_kbps: f64,
}
pub fn eligible(item: &LibraryItem, available: f64) -> bool {
    item.provider.as_deref() == Some("youtube")
        && !item.missing
        && item.source_abr_kbps.is_some_and(|current| {
            current.is_finite()
                && current >= 0.0
                && available.is_finite()
                && available >= current + 40.0
        })
}
pub async fn scan(
    db: &Db,
    backend: &dyn DownloadBackend,
    ids: Option<Vec<i64>>,
    cancel: &CancellationToken,
) -> CoreResult<Vec<UpgradeCandidate>> {
    let items=db.call(move|conn|{
        let mut stmt=conn.prepare("SELECT id FROM library WHERE provider='youtube' AND source_abr_kbps IS NOT NULL AND missing=0")?;
        let values=stmt.query_map([],|r|r.get::<_,i64>(0))?.collect::<Result<Vec<_>,_>>()?;
        values.into_iter().filter(|id|ids.as_ref().is_none_or(|ids|ids.contains(id))).map(|id|crate::library::get(conn,id)?.ok_or_else(||crate::CoreError::invalid("Missing item"))).collect::<CoreResult<Vec<_>>>()
    }).await?;
    let mut out = Vec::new();
    for item in items {
        if cancel.is_cancelled() {
            return Err(crate::CoreError::coded("cancelled", "Cancelled"));
        }
        let Some(url) = item.source_url.as_deref() else {
            continue;
        };
        match backend.analyze(url, cancel).await {
            Ok(Analysis::Video { info }) => {
                if let Some(available) = info.best_audio_abr.filter(|abr| eligible(&item, *abr)) {
                    out.push(UpgradeCandidate {
                        item,
                        available_abr_kbps: available,
                    });
                }
            }
            Err(error) => {
                return Err(crate::CoreError::coded(
                    error.kind.as_str(),
                    crate::logging::redact(&error.message),
                ))
            }
            _ => {}
        }
    }
    Ok(out)
}

pub async fn run(
    db: &Db,
    backend: &dyn DownloadBackend,
    data_dir: &Path,
    ffmpeg_dir: &Path,
    job: &PipelineJob,
    cancel: &CancellationToken,
    on_event: &(dyn Fn(PipelineEvent) + Send + Sync),
) -> Result<PipelineOutput, DownloadError> {
    let disk = |e: crate::CoreError| DownloadError::new(ErrorKind::Disk, e.to_string());
    let id = job
        .options
        .upgrade_library_id
        .ok_or_else(|| DownloadError::new(ErrorKind::Unknown, "Upgrade ID missing"))?;
    let item = db
        .call(move |conn| crate::library::get(conn, id))
        .await
        .map_err(disk)?
        .ok_or_else(|| DownloadError::new(ErrorKind::Unavailable, "Library item missing"))?;
    let target = PathBuf::from(&item.file_path);
    crate::tagging::read_tags(&target).map_err(disk)?;
    let url = item
        .source_url
        .as_ref()
        .ok_or_else(|| DownloadError::new(ErrorKind::Unavailable, "Source URL missing"))?;
    let analysis = backend.analyze(url, cancel).await?;
    let Analysis::Video { info } = analysis else {
        return Err(DownloadError::new(
            ErrorKind::Unavailable,
            "Source is not a video",
        ));
    };
    if !info.best_audio_abr.is_some_and(|abr| eligible(&item, abr)) {
        return Err(DownloadError::new(
            ErrorKind::Unavailable,
            "No quality improvement available",
        ));
    }
    let workspace = crate::workspace::JobWorkspace::new(data_dir, &job.job_id).map_err(disk)?;
    let done = backend
        .download(
            &JobSpec {
                url: url.clone(),
                tmp_dir: workspace.path().to_owned(),
                sponsorblock: None,
            },
            &|p| on_event(PipelineEvent::Download(p)),
            cancel,
        )
        .await?;
    if !done.abr.is_some_and(|abr| eligible(&item, abr)) {
        return Err(DownloadError::new(
            ErrorKind::Unavailable,
            "Downloaded source did not improve quality",
        ));
    }
    let profile = job.profile.resolve(&done.ext);
    let ready = workspace.path().join(format!(
        "upgrade.{}",
        target.extension().unwrap_or_default().to_string_lossy()
    ));
    let downloaded = PathBuf::from(&done.filepath);
    // Um item criado por divisão continua sendo apenas a sua faixa no upgrade.
    let chapter = if info.duration.is_some_and(|d| d > 600.0)
        && item.track_total == Some(info.chapters.len() as u32)
        && info.chapters.len() >= 2
    {
        item.track_no
            .and_then(|n| n.checked_sub(1))
            .and_then(|n| info.chapters.get(n as usize))
    } else {
        None
    };
    let input = if let Some(chapter) = chapter {
        let segment = workspace
            .path()
            .join(format!("upgrade-source.{}", done.ext));
        crate::quality::audio::segment(
            &ffmpeg_dir.join(crate::quality::exe("ffmpeg")),
            &downloaded,
            &segment,
            chapter.start_time,
            chapter.end_time,
            &crate::profiles::profile("original")
                .expect("original profile")
                .resolve(&done.ext),
            cancel,
        )
        .await?;
        segment
    } else {
        downloaded
    };
    let mut codec = profile.clone();
    if !codec.needs_conversion() {
        codec.args = if done.ext == ready.extension().unwrap_or_default().to_string_lossy() {
            vec!["-c:a".into(), "copy".into()]
        } else {
            crate::quality::audio::codec_args(
                &codec,
                &ready.extension().unwrap_or_default().to_string_lossy(),
                done.abr.unwrap_or(256.0),
            )
        };
    }
    crate::transcode::convert(
        &ffmpeg_dir.join(crate::quality::exe("ffmpeg")),
        &input,
        &ready,
        &codec,
        done.duration.unwrap_or(0.0),
        cancel,
        &|p| on_event(PipelineEvent::Convert(p)),
    )
    .await?;
    let mut tags = crate::tagging::read_tags(&target).map_err(disk)?;
    let gain = crate::loudness::analyze(
        &ffmpeg_dir.join(crate::quality::exe("ffmpeg")),
        &ready,
        cancel,
    )
    .await?
    .replay_gain()
    .map_err(disk)?;
    tags.set_replay_gain(&gain, ready.extension().is_some_and(|e| e == "opus"));
    crate::tagging::write_tags(&ready, &tags).map_err(disk)?;
    let probe =
        crate::transcode::probe(&ffmpeg_dir.join(crate::quality::exe("ffprobe")), &ready).await?;
    if cancel.is_cancelled() {
        return Err(DownloadError::cancelled());
    }
    let replacement =
        crate::quality::replace::Replacement::publish(&ready, &target).map_err(disk)?;
    let file = DownloadedFile {
        file_path: item.file_path,
        tags,
        probe: Some(probe),
        source_abr_kbps: done.abr,
        content_type: item.content_type,
        has_synced_lyrics: item.has_synced_lyrics,
        cover_source: item.cover_source,
        replaygain_db: Some(gain.track_gain_db),
    };
    Ok(PipelineOutput {
        path: target,
        done,
        profile,
        metadata: None,
        library_file: Some(file),
        publication: None,
        chapter_files: vec![],
        chapter_publications: vec![],
        replacement: Some(replacement),
        upgrade_id: Some(id),
    })
}

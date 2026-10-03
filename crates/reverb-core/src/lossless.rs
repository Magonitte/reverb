//! Spectral heuristic, never a proof of provenance. Temporary images are removed on return.
use crate::ytdlp::DownloadError;
use crate::{CoreError, CoreResult, Db};
use base64::Engine;
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LosslessVerdict {
    Lossless,
    #[serde(rename = "lossy_16k")]
    Lossy16k,
    #[serde(rename = "lossy_19k")]
    Lossy19k,
    Inconclusive,
}
impl LosslessVerdict {
    pub fn id(&self) -> &'static str {
        match self {
            Self::Lossless => "lossless",
            Self::Lossy16k => "lossy_16k",
            Self::Lossy19k => "lossy_19k",
            Self::Inconclusive => "inconclusive",
        }
    }
}
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LosslessReport {
    pub verdict: LosslessVerdict,
    pub details: String,
    pub spectrogram_png_base64: String,
    pub rms_db: f64,
    pub high16_db: f64,
    pub high19_db: f64,
}

fn core(e: DownloadError) -> CoreError {
    CoreError::coded(e.kind.as_str(), e.message)
}
async fn rms(
    ffmpeg: &Path,
    path: &Path,
    filter: String,
    cancel: &CancellationToken,
) -> CoreResult<f64> {
    let mut filter = filter;
    filter.push_str("astats=metadata=0:reset=0");
    let args = vec![
        "-hide_banner".into(),
        "-nostdin".into(),
        "-i".into(),
        path.to_string_lossy().into_owned(),
        "-map".into(),
        "0:a:0".into(),
        "-vn".into(),
        "-af".into(),
        filter,
        "-f".into(),
        "null".into(),
        "-".into(),
    ];
    let output = tokio::select! {_=cancel.cancelled()=>return Err(CoreError::coded("cancelled","Cancelled")),r=crate::tools::process::run_capture(ffmpeg,args, |_|{},Duration::from_secs(300))=>r?};
    if !output.success {
        return Err(CoreError::coded("ffmpeg", "Cannot measure audio spectrum"));
    }
    output
        .stderr
        .lines()
        .filter_map(|l| l.split_once("RMS level dB:"))
        .filter_map(|(_, s)| s.trim().parse::<f64>().ok())
        .next_back()
        .ok_or_else(|| CoreError::coded("ffmpeg", "Missing RMS level"))
}

pub fn verdict(
    sample_rate: Option<u32>,
    total: f64,
    high16: f64,
    high19: f64,
    drop16: f64,
    drop19: f64,
) -> LosslessVerdict {
    if sample_rate.is_none_or(|s| s < 44100) || !total.is_finite() || total < -80.0 {
        return LosslessVerdict::Inconclusive;
    }
    // Low treble energy alone is normal for acoustic music. Require a sharp drop
    // between neighboring bands as well, rather than labeling a smooth rolloff lossy.
    if high16 - total < -25.0 && drop16 < -15.0 {
        LosslessVerdict::Lossy16k
    } else if high19 - total < -25.0 && drop19 < -15.0 {
        LosslessVerdict::Lossy19k
    } else {
        LosslessVerdict::Lossless
    }
}
pub async fn verify(
    ffmpeg: &Path,
    ffprobe: &Path,
    path: &Path,
    cancel: &CancellationToken,
) -> CoreResult<LosslessReport> {
    let probe = crate::transcode::probe(ffprobe, path).await.map_err(core)?;
    let total = rms(ffmpeg, path, String::new(), cancel).await?;
    let high16 = rms(ffmpeg, path, "highpass=f=16500:p=2,".repeat(6), cancel).await?;
    let high19 = rms(ffmpeg, path, "highpass=f=19500:p=2,".repeat(6), cancel).await?;
    let mut drop16 = 0.0;
    let mut drop19 = 0.0;
    if probe.sample_rate.is_some_and(|s| s >= 44100) && total.is_finite() && total >= -80.0 {
        if high16 - total < -25.0 {
            let before = rms(
                ffmpeg,
                path,
                "bandpass=f=15000:width_type=h:width=1000,".repeat(6),
                cancel,
            )
            .await?;
            let after = rms(
                ffmpeg,
                path,
                "bandpass=f=17500:width_type=h:width=1000,".repeat(6),
                cancel,
            )
            .await?;
            drop16 = after - before;
        }
        if high19 - total < -25.0 {
            let before = rms(
                ffmpeg,
                path,
                "bandpass=f=18500:width_type=h:width=1000,".repeat(6),
                cancel,
            )
            .await?;
            let after = rms(
                ffmpeg,
                path,
                "bandpass=f=20500:width_type=h:width=1000,".repeat(6),
                cancel,
            )
            .await?;
            drop19 = after - before;
        }
    }
    let temp = tempfile::tempdir()?;
    let png = temp.path().join("spectrum.png");
    crate::quality::audio::execute(
        ffmpeg,
        vec![
            "-hide_banner".into(),
            "-nostdin".into(),
            "-y".into(),
            "-i".into(),
            path.to_string_lossy().into_owned(),
            "-lavfi".into(),
            "showspectrumpic=s=800x300:legend=0".into(),
            "-frames:v".into(),
            "1".into(),
            png.to_string_lossy().into_owned(),
        ],
        cancel,
    )
    .await
    .map_err(core)?;
    let image = tokio::fs::read(png).await?;
    let verdict = verdict(probe.sample_rate, total, high16, high19, drop16, drop19);
    let finite = |n: f64| if n.is_finite() { n } else { -200.0 };
    Ok(LosslessReport {
        verdict,
        details: format!(
            "{} Hz; RMS {:.1} dB; >16.5 kHz {:.1} dB; >19.5 kHz {:.1} dB; band drops {:.1}/{:.1} dB",
            probe.sample_rate.unwrap_or(0),
            finite(total),
            finite(high16),
            finite(high19), finite(drop16), finite(drop19)
        ),
        spectrogram_png_base64: base64::engine::general_purpose::STANDARD.encode(image),
        rms_db: finite(total),
        high16_db: finite(high16),
        high19_db: finite(high19),
    })
}

pub async fn verify_library(
    db: &Db,
    ffmpeg: &Path,
    ffprobe: &Path,
    id: i64,
) -> CoreResult<LosslessReport> {
    let item = db
        .call(move |c| crate::library::get(c, id))
        .await?
        .ok_or_else(|| CoreError::invalid("Library item missing"))?;
    let report = verify(
        ffmpeg,
        ffprobe,
        Path::new(&item.file_path),
        &CancellationToken::new(),
    )
    .await?;
    let verdict = report.verdict.id().to_string();
    db.call(move |c| {
        c.execute(
            "UPDATE library SET lossless_verdict=?,updated_at=unixepoch() WHERE id=?",
            rusqlite::params![verdict, id],
        )?;
        Ok(())
    })
    .await?;
    Ok(report)
}

pub async fn verify_imports(
    db: &Db,
    roots: Vec<PathBuf>,
    ffmpeg: &Path,
    ffprobe: &Path,
) -> CoreResult<Vec<crate::library::files::ImportFailure>> {
    let items = db.call(move |c| {
        let mut statement = c.prepare("SELECT id,file_path FROM library WHERE origin IN ('import','scan') AND missing=0 AND (codec='flac' OR codec LIKE 'pcm_%') AND lossless_verdict IS NULL")?;
        let rows = statement.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows.into_iter().filter(|(_, path)| roots.iter().any(|root| Path::new(path).starts_with(root))).collect::<Vec<_>>())
    }).await?;
    let mut failures = Vec::new();
    for (id, path) in items {
        if let Err(e) = verify_library(db, ffmpeg, ffprobe, id).await {
            failures.push(crate::library::files::ImportFailure {
                path,
                message: e.to_string(),
            });
        }
    }
    Ok(failures)
}

#[cfg(test)]
mod tests;

//! Qualidade da fonte e teste de cookies (F13).
pub mod audio;
use crate::backend::DownloadBackend;
use crate::ytdlp::errors::ErrorKind;
use crate::ytdlp::{Analysis, VideoInfo};
use serde::Serialize;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CookiesTestResult {
    pub ok: bool,
    pub premium: bool,
    pub best_audio: Option<String>,
    pub error_kind: Option<String>,
}

pub fn is_premium(video: &VideoInfo) -> bool {
    video.audio_formats.iter().any(|f| {
        matches!(f.format_id.as_str(), "141" | "774")
            || f.abr.is_some_and(|abr| abr.is_finite() && abr >= 200.0)
    })
}

pub async fn test_cookies(
    backend: &dyn DownloadBackend,
    cancel: &CancellationToken,
) -> CookiesTestResult {
    match backend
        .analyze("https://music.youtube.com/watch?v=lYBUbBu4W08", cancel)
        .await
    {
        Ok(Analysis::Video { info }) => {
            let best = info
                .audio_formats
                .iter()
                .max_by(|a, b| a.abr.unwrap_or(0.0).total_cmp(&b.abr.unwrap_or(0.0)));
            CookiesTestResult {
                ok: true,
                premium: is_premium(&info),
                best_audio: best.map(|f| {
                    format!(
                        "{} {} kbps",
                        if f.acodec.starts_with("mp4a") {
                            "AAC"
                        } else {
                            &f.acodec
                        },
                        f.abr.unwrap_or(0.0).round()
                    )
                }),
                error_kind: None,
            }
        }
        Ok(_) => CookiesTestResult {
            ok: false,
            premium: false,
            best_audio: None,
            error_kind: Some(ErrorKind::Unknown.as_str().into()),
        },
        Err(error) => CookiesTestResult {
            ok: false,
            premium: false,
            best_audio: None,
            error_kind: Some(error.kind.as_str().into()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn premium_requires_audio_format_or_bitrate() {
        for (format, abr, premium) in [
            ("140", 129, false),
            ("141", 0, true),
            ("774", 0, true),
            ("251", 200, true),
        ] {
            let video=VideoInfo::from_json(&format!(r#"{{"id":"x","formats":[{{"format_id":"{format}","abr":{abr},"acodec":"opus","vcodec":"none","ext":"opus"}}]}}"#)).unwrap();
            assert_eq!(is_premium(&video), premium);
        }
        let video=VideoInfo::from_json(r#"{"id":"x","formats":[{"format_id":"141","abr":256,"acodec":"aac","vcodec":"avc1"}]}"#).unwrap();
        assert!(!is_premium(&video));
    }
}

#[cfg(test)]
mod cookies_tests {
    use super::*;
    use crate::queue::fake::FakeBackend;
    #[tokio::test]
    async fn cookies_test_uses_backend_and_reports_best_audio() {
        let backend = FakeBackend::default();
        let video=VideoInfo::from_json(r#"{"id":"lYBUbBu4W08","formats":[{"format_id":"141","abr":256,"acodec":"mp4a.40.2","vcodec":"none","ext":"m4a"}]}"#).unwrap();
        backend.set_video(video);
        let result = test_cookies(&backend, &CancellationToken::new()).await;
        assert!(result.ok && result.premium);
        assert_eq!(result.best_audio.as_deref(), Some("AAC 256 kbps"));
    }
}

pub mod chapters;
pub mod replace;

pub mod providers;

pub mod acoustid;

pub mod upgrade;
pub async fn trim(
    db: &crate::Db,
    ffmpeg: &std::path::Path,
    ffprobe: &std::path::Path,
    path: &std::path::Path,
    start: f64,
    end: f64,
    cancel: &CancellationToken,
) -> crate::CoreResult<()> {
    let path = crate::library::files::canonical(path)?;
    let probe = crate::transcode::probe(ffprobe, &path)
        .await
        .map_err(|e| crate::CoreError::coded(e.kind.as_str(), e.message))?;
    if !start.is_finite()
        || !end.is_finite()
        || start < 0.0
        || end <= start
        || end > probe.duration_s + 0.05
    {
        return Err(crate::CoreError::invalid("Invalid audio range"));
    }
    let tags = crate::tagging::read_tags(&path)?;
    let dir = tempfile::Builder::new()
        .prefix(".reverb-trim-")
        .tempdir_in(
            path.parent()
                .ok_or_else(|| crate::CoreError::invalid("Missing parent"))?,
        )?;
    let ext = path.extension().unwrap_or_default().to_string_lossy();
    let ready = dir.path().join(format!("trim.{ext}"));
    let profile = crate::profiles::profile("original")
        .expect("original profile")
        .resolve(&ext);
    audio::segment(ffmpeg, &path, &ready, start, end, &profile, cancel)
        .await
        .map_err(|e| crate::CoreError::coded(e.kind.as_str(), e.message))?;
    crate::tagging::write_tags(&ready, &tags)?;
    let probe = crate::transcode::probe(ffprobe, &ready)
        .await
        .map_err(|e| crate::CoreError::coded(e.kind.as_str(), e.message))?;
    if cancel.is_cancelled() {
        return Err(crate::CoreError::coded("cancelled", "Cancelled"));
    }
    let mut replacement = replace::Replacement::publish(&ready, &path)?;
    let target = path.to_string_lossy().into_owned();
    db.call(move|conn|{let tx=conn.transaction()?;crate::library::files::update_tags(&tx,&target,&tags)?;tx.execute("UPDATE library SET duration_s=?,bitrate_kbps=?,updated_at=unixepoch() WHERE file_path=?",rusqlite::params![probe.duration_s,probe.bitrate_kbps,target])?;tx.commit()?;Ok(())}).await?;
    replacement.commit()?;
    Ok(())
}
pub fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.into()
    }
}

pub fn png_url(bytes: &[u8]) -> String {
    use base64::Engine;
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

#[cfg(test)]
#[path = "quality/tests.rs"]
mod advanced_tests;

pub mod import;

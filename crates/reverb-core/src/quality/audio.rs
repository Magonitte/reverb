//! Cortes não destrutivos até a publicação e geração de forma de onda.
use crate::exec::{run_streaming, ExecFailure, ExecSpec};
use crate::profiles::ResolvedProfile;
use crate::ytdlp::{DownloadError, ErrorKind};
use std::path::Path;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

// Only remove the initial silence in each direction; stop_periods truncates internal pauses.
pub const SILENCE_FILTER: &str = "silenceremove=start_periods=1:start_threshold=-50dB:start_silence=0,areverse,silenceremove=start_periods=1:start_threshold=-50dB:start_silence=0,areverse";

pub fn codec_args(profile: &ResolvedProfile, ext: &str, abr: f64) -> Vec<String> {
    if profile.needs_conversion() {
        return profile.args.clone();
    }
    let codec = match ext {
        "opus" => "libopus",
        "ogg" => "libvorbis",
        "m4a" => "aac",
        "mp3" => "libmp3lame",
        "flac" => "flac",
        _ => "pcm_s16le",
    };
    let mut args = vec!["-c:a".into(), codec.into()];
    if !matches!(ext, "flac" | "wav") {
        args.extend(["-b:a".into(), format!("{}k", abr.max(160.0).ceil())]);
    }
    args
}

pub async fn execute(
    ffmpeg: &Path,
    args: Vec<String>,
    cancel: &CancellationToken,
) -> Result<(), DownloadError> {
    let spec = ExecSpec {
        program: ffmpeg.to_owned(),
        args,
        env: vec![],
        idle_timeout: Some(Duration::from_secs(120)),
        total_timeout: Some(Duration::from_secs(3600)),
    };
    let result = run_streaming(spec, cancel, &mut |_| {})
        .await
        .map_err(|e| match e {
            ExecFailure::Cancelled => DownloadError::cancelled(),
            _ => DownloadError::new(ErrorKind::Ffmpeg, format!("{e:?}")),
        })?;
    if !result.success {
        return Err(DownloadError::from_stderr(
            result.stderr_tail,
            "FFmpeg failed",
        ));
    }
    Ok(())
}

pub async fn segment(
    ffmpeg: &Path,
    input: &Path,
    output: &Path,
    start: f64,
    end: f64,
    profile: &ResolvedProfile,
    cancel: &CancellationToken,
) -> Result<(), DownloadError> {
    if !start.is_finite() || !end.is_finite() || start < 0.0 || end <= start {
        return Err(DownloadError::new(ErrorKind::Ffmpeg, "Invalid audio range"));
    }
    let mut args = vec![
        "-hide_banner".into(),
        "-nostdin".into(),
        "-y".into(),
        "-i".into(),
        input.to_string_lossy().into_owned(),
        "-ss".into(),
        start.to_string(),
        "-to".into(),
        end.to_string(),
        "-map".into(),
        "0:a:0".into(),
        "-vn".into(),
    ];
    args.extend(if profile.needs_conversion() {
        profile.args.clone()
    } else {
        vec!["-c:a".into(), "copy".into()]
    });
    args.push(output.to_string_lossy().into_owned());
    execute(ffmpeg, args, cancel).await
}

pub async fn silence(
    ffmpeg: &Path,
    input: &Path,
    output: &Path,
    profile: &ResolvedProfile,
    abr: f64,
    cancel: &CancellationToken,
) -> Result<(), DownloadError> {
    let mut args = vec![
        "-hide_banner".into(),
        "-nostdin".into(),
        "-y".into(),
        "-i".into(),
        input.to_string_lossy().into_owned(),
        "-map".into(),
        "0:a:0".into(),
        "-vn".into(),
        "-af".into(),
        SILENCE_FILTER.into(),
    ];
    args.extend(codec_args(profile, &profile.ext, abr));
    args.push(output.to_string_lossy().into_owned());
    execute(ffmpeg, args, cancel).await
}

pub async fn waveform(
    ffmpeg: &Path,
    input: &Path,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, DownloadError> {
    let dir =
        tempfile::tempdir().map_err(|e| DownloadError::new(ErrorKind::Disk, e.to_string()))?;
    let out = dir.path().join("wave.png");
    execute(
        ffmpeg,
        vec![
            "-hide_banner".into(),
            "-nostdin".into(),
            "-y".into(),
            "-i".into(),
            input.to_string_lossy().into_owned(),
            "-filter_complex".into(),
            "showwavespic=s=1200x160:colors=0xf59e4b".into(),
            "-frames:v".into(),
            "1".into(),
            out.to_string_lossy().into_owned(),
        ],
        cancel,
    )
    .await?;
    std::fs::read(out).map_err(|e| DownloadError::new(ErrorKind::Disk, e.to_string()))
}

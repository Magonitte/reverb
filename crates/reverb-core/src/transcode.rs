//! Conversão por perfil com ffmpeg (progresso por `-progress pipe:1`) e `probe` com ffprobe
//! (arquitetura §7).

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

use crate::exec::{run_streaming, ExecFailure, ExecSpec};
use crate::profiles::ResolvedProfile;
use crate::tools::process::run_capture;
use crate::ytdlp::errors::{classify_stderr, DownloadError, ErrorKind};

const PROBE_TIMEOUT: Duration = Duration::from_secs(60);

/// O que o ffprobe diz do primeiro stream de áudio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProbeInfo {
    pub codec: String,
    pub bitrate_kbps: Option<u32>,
    pub duration_s: f64,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
}

fn number(value: Option<&serde_json::Value>) -> Option<f64> {
    let value = value?;
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}

/// Interpreta a saída JSON do ffprobe (`-show_format -show_streams`).
pub fn parse_probe(json: &str) -> Result<ProbeInfo, DownloadError> {
    let invalid = |why: &str| {
        DownloadError::new(
            ErrorKind::Ffmpeg,
            format!("saída do ffprobe inválida: {why}"),
        )
    };
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| invalid(&e.to_string()))?;
    let stream = value["streams"]
        .as_array()
        .and_then(|streams| {
            streams
                .iter()
                .find(|s| s["codec_type"].as_str() == Some("audio"))
        })
        .ok_or_else(|| invalid("nenhum stream de áudio"))?;
    let format = &value["format"];
    let duration_s = number(stream.get("duration"))
        .filter(|d| *d > 0.0)
        .or_else(|| number(format.get("duration")))
        .ok_or_else(|| invalid("sem duração"))?;
    let bitrate = number(stream.get("bit_rate")).or_else(|| number(format.get("bit_rate")));
    Ok(ProbeInfo {
        codec: stream["codec_name"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        bitrate_kbps: bitrate.map(|bps| (bps / 1000.0).round() as u32),
        duration_s,
        sample_rate: number(stream.get("sample_rate")).map(|n| n as u32),
        channels: number(stream.get("channels")).map(|n| n as u32),
    })
}

/// `ffprobe -v error -print_format json -show_format -show_streams <arquivo>`.
pub async fn probe(ffprobe: &Path, file: &Path) -> Result<ProbeInfo, DownloadError> {
    let args = [
        "-v".to_string(),
        "error".to_string(),
        "-print_format".to_string(),
        "json".to_string(),
        "-show_format".to_string(),
        "-show_streams".to_string(),
        file.to_string_lossy().into_owned(),
    ];
    let output = run_capture(ffprobe, args, |_| {}, PROBE_TIMEOUT)
        .await
        .map_err(|e| {
            DownloadError::new(
                ErrorKind::Ffmpeg,
                format!("não foi possível rodar o ffprobe: {e}"),
            )
        })?;
    if !output.success {
        let tail: Vec<String> = output.stderr.lines().map(str::to_string).collect();
        let mut error = DownloadError::from_stderr(tail, "o ffprobe falhou");
        error.kind = ErrorKind::Ffmpeg;
        return Err(error);
    }
    parse_probe(&output.stdout)
}

/// Argumentos da conversão (§7): `-hide_banner -nostdin -y -i <in> -map 0:a:0 -vn -map_metadata -1
/// <codec> -progress pipe:1 -nostats <out>`.
pub fn convert_args(input: &Path, output: &Path, profile: &ResolvedProfile) -> Vec<String> {
    let mut args: Vec<String> = ["-hide_banner", "-nostdin", "-y", "-i"]
        .map(String::from)
        .to_vec();
    args.push(input.to_string_lossy().into_owned());
    args.extend(["-map", "0:a:0", "-vn", "-map_metadata", "-1"].map(String::from));
    args.extend(profile.args.iter().cloned());
    args.extend(["-progress", "pipe:1", "-nostats"].map(String::from));
    args.push(output.to_string_lossy().into_owned());
    args
}

/// Progresso (0–100) a partir de uma linha do `-progress`; `None` para as demais linhas.
/// `out_time_us` (e o `out_time_ms`, que na prática também é em µs) dividido pela duração.
pub fn parse_progress_percent(line: &str, duration_s: f64) -> Option<u8> {
    if line.trim() == "progress=end" {
        return Some(100);
    }
    let micros: f64 = line
        .strip_prefix("out_time_us=")
        .or_else(|| line.strip_prefix("out_time_ms="))?
        .trim()
        .parse()
        .ok()?;
    if duration_s <= 0.0 || micros < 0.0 {
        return None;
    }
    Some(((micros / (duration_s * 1_000_000.0)) * 100.0).clamp(0.0, 100.0) as u8)
}

/// Converte `input` para `output` conforme o perfil. `duration_s` vem do `probe` da entrada.
/// `on_progress` recebe percentuais crescentes (0–100).
pub async fn convert(
    ffmpeg: &Path,
    input: &Path,
    output: &Path,
    profile: &ResolvedProfile,
    duration_s: f64,
    cancel: &CancellationToken,
    on_progress: &(dyn Fn(u8) + Send + Sync),
) -> Result<(), DownloadError> {
    let spec = ExecSpec {
        program: ffmpeg.to_path_buf(),
        args: convert_args(input, output, profile),
        env: Vec::new(),
        idle_timeout: Some(Duration::from_secs(300)),
        total_timeout: None,
    };
    let mut last = 0u8;
    let mut on_line = |line: &str| {
        if let Some(percent) = parse_progress_percent(line, duration_s) {
            if percent > last {
                last = percent;
                on_progress(percent);
            }
        }
    };
    let outcome = match run_streaming(spec, cancel, &mut on_line).await {
        Ok(outcome) => outcome,
        Err(ExecFailure::Cancelled) => return Err(DownloadError::cancelled()),
        Err(ExecFailure::Idle | ExecFailure::Timeout) => {
            return Err(DownloadError::new(
                ErrorKind::Ffmpeg,
                "o ffmpeg parou de responder",
            ))
        }
        Err(ExecFailure::Spawn(e)) => {
            return Err(DownloadError::new(
                ErrorKind::Ffmpeg,
                format!("não foi possível iniciar o ffmpeg: {e}"),
            ))
        }
    };
    if !outcome.success {
        let kind = match classify_stderr(&outcome.stderr_tail) {
            ErrorKind::Disk => ErrorKind::Disk,
            _ => ErrorKind::Ffmpeg,
        };
        let mut error = DownloadError::from_stderr(outcome.stderr_tail, "o ffmpeg falhou");
        error.kind = kind;
        return Err(error);
    }
    if last < 100 {
        on_progress(100);
    }
    Ok(())
}

#[cfg(test)]
mod tests;

//! Análise EBU R128 sem alterar o áudio (F09, arquitetura §12).

use std::path::Path;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::exec::{run_streaming, ExecFailure, ExecSpec};
use crate::ytdlp::errors::{DownloadError, ErrorKind};
use crate::{CoreError, CoreResult};

/// Medições do resumo final de `ebur128=peak=true`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loudness {
    pub integrated_lufs: f64,
    /// `-inf` representa pico zero (silêncio).
    pub true_peak_dbfs: f64,
}

/// Valores numéricos para as tags da tarefa 4. Não aplicam ganho ao áudio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReplayGain {
    pub track_gain_db: f64,
    pub track_peak: f64,
    /// Ganho Opus Q7.8, inteiro com sinal de 16 bits (RFC 7845 §5.2).
    pub r128_track_gain: i16,
}

impl Loudness {
    pub fn replay_gain(self) -> CoreResult<ReplayGain> {
        if !self.integrated_lufs.is_finite()
            || !(self.true_peak_dbfs.is_finite() || self.true_peak_dbfs == f64::NEG_INFINITY)
        {
            return Err(invalid("medição não finita"));
        }
        let track_gain_db = -18.0 - self.integrated_lufs;
        let track_peak = 10.0_f64.powf(self.true_peak_dbfs / 20.0);
        let r128 = ((-23.0 - self.integrated_lufs) * 256.0).round();
        if !track_peak.is_finite() || !(f64::from(i16::MIN)..=f64::from(i16::MAX)).contains(&r128) {
            return Err(invalid("ganho ou pico fora da faixa representável"));
        }
        Ok(ReplayGain {
            track_gain_db,
            track_peak,
            r128_track_gain: r128 as i16,
        })
    }
}

impl ReplayGain {
    /// `REPLAYGAIN_TRACK_GAIN`: duas casas decimais e unidade dB.
    pub fn track_gain_tag(&self) -> String {
        // Evita `-0.00 dB` quando o valor arredonda para zero.
        let gain = if self.track_gain_db.abs() < 0.005 {
            0.0
        } else {
            self.track_gain_db
        };
        format!("{gain:.2} dB")
    }

    /// `REPLAYGAIN_TRACK_PEAK`: amplitude linear com seis casas decimais.
    pub fn track_peak_tag(&self) -> String {
        format!("{:.6}", self.track_peak)
    }

    /// `R128_TRACK_GAIN`: inteiro decimal, usado na saída Opus.
    pub fn r128_track_gain_tag(&self) -> String {
        self.r128_track_gain.to_string()
    }
}

fn invalid(reason: &str) -> CoreError {
    CoreError::coded(
        "loudness_parse",
        format!("resumo EBU R128 inválido: {reason}"),
    )
}

/// Métrica em linha própria, tolerando espaços, CRLF e prefixos do logger do FFmpeg.
fn metric(section: &str, key: &str, unit: &str) -> CoreResult<f64> {
    for line in section.lines() {
        let mut line = line.trim();
        while let Some(rest) = line.strip_prefix('[') {
            let Some((_, rest)) = rest.split_once(']') else {
                break;
            };
            line = rest.trim_start();
        }
        if let Some(value) = line.strip_prefix(key) {
            let mut parts = value.split_whitespace();
            let number = parts.next().and_then(|n| n.parse::<f64>().ok());
            if parts.next() != Some(unit) || parts.next().is_some() {
                return Err(invalid("unidade inesperada"));
            }
            return number.ok_or_else(|| invalid("valor numérico ausente ou inválido"));
        }
    }
    Err(invalid("métrica ausente"))
}

/// Usa o último `Summary:` (algumas saídas também têm um resumo inicial vazio).
/// Leituras por frame, thresholds e sample peaks não substituem o resumo final.
pub fn parse_summary(stderr: &str) -> CoreResult<Loudness> {
    let (_, summary) = stderr
        .rsplit_once("Summary:")
        .ok_or_else(|| invalid("sem Summary"))?;
    let (_, integrated) = summary
        .split_once("Integrated loudness:")
        .ok_or_else(|| invalid("sem Integrated loudness"))?;
    let (integrated, true_peak) = integrated
        .split_once("True peak:")
        .ok_or_else(|| invalid("sem True peak"))?;
    let loudness = Loudness {
        integrated_lufs: metric(integrated, "I:", "LUFS")?,
        true_peak_dbfs: metric(true_peak, "Peak:", "dBFS")?,
    };
    loudness.replay_gain()?;
    Ok(loudness)
}

pub fn analyze_args(input: &Path) -> Vec<String> {
    let mut args = [
        "-hide_banner",
        "-nostdin",
        "-nostats",
        "-loglevel",
        "info",
        "-i",
    ]
    .map(String::from)
    .to_vec();
    args.push(input.to_string_lossy().into_owned());
    args.extend(
        [
            "-map",
            "0:a:0",
            "-vn",
            "-af",
            "ebur128=peak=true",
            "-f",
            "null",
            "-",
        ]
        .map(String::from),
    );
    args
}

/// Mede o primeiro stream de áudio; stdout descartado, resumo lido do stderr.
/// Usa o executor comum para cancelamento, watchdog e encerramento da árvore.
pub async fn analyze(
    ffmpeg: &Path,
    input: &Path,
    cancel: &CancellationToken,
) -> Result<Loudness, DownloadError> {
    let spec = ExecSpec {
        program: ffmpeg.to_path_buf(),
        args: analyze_args(input),
        env: vec![("AV_LOG_FORCE_NOCOLOR".into(), "1".into())],
        idle_timeout: Some(Duration::from_secs(300)),
        total_timeout: None,
    };
    let output = run_streaming(spec, cancel, &mut |_| {})
        .await
        .map_err(|error| match error {
            ExecFailure::Cancelled => DownloadError::cancelled(),
            ExecFailure::Idle | ExecFailure::Timeout => DownloadError::new(
                ErrorKind::Ffmpeg,
                "o ffmpeg parou de responder durante a análise de loudness",
            ),
            ExecFailure::Spawn(error) => DownloadError::new(
                ErrorKind::Ffmpeg,
                format!("não foi possível iniciar a análise de loudness: {error}"),
            ),
        })?;
    if !output.success {
        let mut error =
            DownloadError::from_stderr(output.stderr_tail, "a análise de loudness falhou");
        error.kind = ErrorKind::Ffmpeg;
        return Err(error);
    }
    parse_summary(&output.stderr_tail.join("\n")).map_err(|error| {
        let mut failure = DownloadError::new(ErrorKind::Ffmpeg, error.to_string());
        failure.stderr_tail = output.stderr_tail;
        failure
    })
}

#[cfg(test)]
mod tests;

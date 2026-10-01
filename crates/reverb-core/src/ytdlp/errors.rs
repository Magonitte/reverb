//! Classificação do stderr do yt-dlp em `ErrorKind` (arquitetura §9) e o erro do motor.

use std::sync::LazyLock;

use regex::{Regex, RegexBuilder};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ErrorKind {
    Cancelled,
    Unavailable,
    AgeRestricted,
    BotCheck,
    GeoBlocked,
    Disk,
    Ffmpeg,
    Extractor,
    Network,
    Unknown,
}

impl ErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::Unavailable => "unavailable",
            Self::AgeRestricted => "age_restricted",
            Self::BotCheck => "bot_check",
            Self::GeoBlocked => "geo_blocked",
            Self::Disk => "disk",
            Self::Ffmpeg => "ffmpeg",
            Self::Extractor => "extractor",
            Self::Network => "network",
            Self::Unknown => "unknown",
        }
    }

    /// Falhas que a fila repete com espera (§9): `network`, `unknown` e `ffmpeg`.
    pub fn is_retryable(self) -> bool {
        matches!(self, Self::Network | Self::Unknown | Self::Ffmpeg)
    }
}

/// Regras na ordem da tabela §9; vale a primeira que casar.
static RULES: LazyLock<Vec<(ErrorKind, Regex)>> = LazyLock::new(|| {
    let rule = |kind, pattern: &str| {
        (
            kind,
            RegexBuilder::new(pattern)
                .case_insensitive(true)
                .build()
                .expect("regex de erro válida"),
        )
    };
    vec![
        // Antes de `Unavailable`: "This video is not available in your country" é bloqueio regional.
        rule(
            ErrorKind::GeoBlocked,
            r"available in your country|geo.?restrict",
        ),
        rule(
            ErrorKind::Unavailable,
            r"Video unavailable|This video is unavailable|Private video|This video has been removed|members-only|This video is not available|HTTP Error 404",
        ),
        rule(
            ErrorKind::AgeRestricted,
            r"Sign in to confirm your age|age-restricted",
        ),
        rule(
            ErrorKind::BotCheck,
            r"Sign in to confirm you.re not a bot|confirm you are not a robot",
        ),
        rule(
            ErrorKind::Disk,
            r"No space left|Permission denied|Access is denied|ENOSPC",
        ),
        rule(
            ErrorKind::Ffmpeg,
            r"ffmpeg not found|ffprobe not found|Postprocessing: .*(Error|failed)",
        ),
        rule(
            ErrorKind::Extractor,
            r"Unable to extract|nsig extraction failed|Signature extraction failed|Requested format is not available|HTTP Error 403|jsc|challenge|Some formats may be missing|No video formats found",
        ),
        rule(
            ErrorKind::Network,
            r"Unable to download webpage|timed out|Connection (reset|refused|aborted)|getaddrinfo failed|Temporary failure in name resolution|HTTP Error 5\d\d|IncompleteRead",
        ),
    ]
});

/// Classifica as linhas de stderr. Sem nenhuma regra casando ⇒ `Unknown`.
pub fn classify_stderr(lines: &[String]) -> ErrorKind {
    let text = lines.join("\n");
    RULES
        .iter()
        .find(|(_, regex)| regex.is_match(&text))
        .map_or(ErrorKind::Unknown, |(kind, _)| *kind)
}

/// Erro de uma execução do motor (yt-dlp ou ffmpeg).
#[derive(Debug, Clone, thiserror::Error)]
#[error("{kind:?}: {message}")]
pub struct DownloadError {
    pub kind: ErrorKind,
    pub message: String,
    pub stderr_tail: Vec<String>,
}

impl DownloadError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            stderr_tail: Vec::new(),
        }
    }

    pub fn cancelled() -> Self {
        Self::new(ErrorKind::Cancelled, "cancelado pelo usuário")
    }

    /// Classifica a partir do stderr; a mensagem é a última linha `ERROR:` (ou a última linha).
    pub fn from_stderr(stderr_tail: Vec<String>, fallback_message: &str) -> Self {
        let kind = classify_stderr(&stderr_tail);
        let message = stderr_tail
            .iter()
            .rev()
            .find(|line| line.trim_start().starts_with("ERROR:"))
            .or_else(|| stderr_tail.iter().rev().find(|l| !l.trim().is_empty()))
            .map_or_else(|| fallback_message.to_string(), |l| l.trim().to_string());
        Self {
            kind,
            message,
            stderr_tail,
        }
    }
}

#[cfg(test)]
mod tests;

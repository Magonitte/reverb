//! Modelo da fila: o `Job` (espelho da tabela `jobs`), estados, estágios e os pedidos da UI
//! (arquitetura §5 e §10).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "queued" => Self::Queued,
            "running" => Self::Running,
            "done" => Self::Done,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => return None,
        })
    }

    pub fn is_finished(self) -> bool {
        matches!(self, Self::Done | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum JobStage {
    Waiting,
    WaitingRetry,
    Analyzing,
    Downloading,
    Converting,
    Metadata,
    Artwork,
    Lyrics,
    Loudness,
    Tagging,
    Moving,
    Done,
}

impl JobStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::WaitingRetry => "waiting_retry",
            Self::Analyzing => "analyzing",
            Self::Downloading => "downloading",
            Self::Converting => "converting",
            Self::Metadata => "metadata",
            Self::Artwork => "artwork",
            Self::Lyrics => "lyrics",
            Self::Loudness => "loudness",
            Self::Tagging => "tagging",
            Self::Moving => "moving",
            Self::Done => "done",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "waiting" => Self::Waiting,
            "waiting_retry" => Self::WaitingRetry,
            "analyzing" => Self::Analyzing,
            "downloading" => Self::Downloading,
            "converting" => Self::Converting,
            "metadata" => Self::Metadata,
            "artwork" => Self::Artwork,
            "lyrics" => Self::Lyrics,
            "loudness" => Self::Loudness,
            "tagging" => Self::Tagging,
            "moving" => Self::Moving,
            "done" => Self::Done,
            _ => return None,
        })
    }

    /// Peso do estágio no progresso geral (§10): download 70, conversão 10, metadados…tags 17,
    /// mover 3. Estágios ausentes redistribuem proporcionalmente.
    fn weight(self) -> f64 {
        match self {
            Self::Downloading => 70.0,
            Self::Converting => 10.0,
            Self::Metadata | Self::Artwork | Self::Lyrics | Self::Loudness | Self::Tagging => 17.0,
            Self::Moving => 3.0,
            _ => 0.0,
        }
    }
}

/// Progresso geral (0–1) de um job que passa pelos estágios `present` (em ordem), estando no
/// estágio `stage` com `stage_progress` (0–1) concluído.
pub fn overall_progress(present: &[JobStage], stage: JobStage, stage_progress: f64) -> f64 {
    let total: f64 = present.iter().map(|s| s.weight()).sum();
    if total <= 0.0 {
        return 0.0;
    }
    let mut before = 0.0;
    for candidate in present {
        if *candidate == stage {
            let inside = candidate.weight() * stage_progress.clamp(0.0, 1.0);
            return ((before + inside) / total).clamp(0.0, 1.0);
        }
        before += candidate.weight();
    }
    0.0
}

/// Sobrescritas por job das configurações globais (todas opcionais).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct JobOptions {
    pub fetch_metadata: Option<bool>,
    pub fetch_lyrics: Option<bool>,
    pub fetch_artwork: Option<bool>,
    pub sponsorblock: Option<bool>,
    pub auto_organize: Option<bool>,
    pub output_dir: Option<String>,
    pub split_chapters: Option<bool>,
}

/// De onde o job veio, quando é item de uma playlist/álbum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, optional_fields)]
pub struct PlaylistCtx {
    pub playlist_title: String,
    pub playlist_id: String,
    pub index: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Job {
    pub id: String,
    pub kind: String,
    pub provider: String,
    pub source_url: String,
    pub source_id: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub thumbnail: Option<String>,
    pub duration_s: Option<f64>,
    pub profile_id: String,
    pub options: JobOptions,
    #[ts(type = "Record<string, unknown> | null")]
    pub metadata_override: Option<serde_json::Value>,
    pub warnings: Vec<String>,
    pub playlist_ctx: Option<PlaylistCtx>,
    pub sync_id: Option<String>,
    pub status: JobStatus,
    pub stage: JobStage,
    /// Progresso do estágio atual (0–1).
    pub progress: f64,
    /// Progresso geral ponderado (0–1), monotônico.
    pub overall_progress: f64,
    pub speed_bps: Option<f64>,
    #[ts(type = "number | null")]
    pub eta_s: Option<i64>,
    pub error_kind: Option<String>,
    pub error_message: Option<String>,
    pub attempts: u32,
    pub output_path: Option<String>,
    #[ts(type = "number | null")]
    pub library_id: Option<i64>,
    #[ts(type = "number")]
    pub position: i64,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    #[ts(type = "number | null")]
    pub finished_at: Option<i64>,
}

/// Pedido para enfileirar (comando `enqueue`).
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct EnqueueRequest {
    pub url: String,
    pub source_id: Option<String>,
    pub title: Option<String>,
    pub thumbnail: Option<String>,
    pub duration_s: Option<f64>,
    pub profile_id: Option<String>,
    pub options: Option<JobOptions>,
    #[ts(type = "Record<string, unknown> | null")]
    pub metadata_override: Option<serde_json::Value>,
    pub playlist_ctx: Option<PlaylistCtx>,
    /// "Baixar agora": entra na frente da fila.
    pub priority: bool,
    pub allow_duplicate: bool,
}

/// Onde colocar um job reordenado.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MoveTarget {
    Before(String),
    After(String),
    Front,
    Back,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DuplicateHit {
    pub source_id: String,
    /// `queue` (já na fila/baixado) ou `library`.
    pub found_in: String,
    pub job_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct QueueState {
    pub paused: bool,
    pub running: u32,
    pub queued: u32,
    pub healing: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progresso_geral_e_monotonico_e_vai_de_0_a_1() {
        let present = [
            JobStage::Downloading,
            JobStage::Converting,
            JobStage::Moving,
        ];
        let mut last = 0.0;
        for (stage, steps) in [
            (JobStage::Downloading, 10),
            (JobStage::Converting, 10),
            (JobStage::Moving, 1),
        ] {
            for i in 0..=steps {
                let value = overall_progress(&present, stage, f64::from(i) / f64::from(steps));
                assert!(value >= last - 1e-9, "{stage:?} {i}: {value} < {last}");
                last = value;
            }
        }
        assert!((last - 1.0).abs() < 1e-9);
    }

    #[test]
    fn estagios_ausentes_redistribuem_o_peso() {
        let sem_conversao = [JobStage::Downloading, JobStage::Moving];
        let fim_do_download = overall_progress(&sem_conversao, JobStage::Downloading, 1.0);
        assert!((fim_do_download - 70.0 / 73.0).abs() < 1e-9);
        let com_conversao = [
            JobStage::Downloading,
            JobStage::Converting,
            JobStage::Moving,
        ];
        let valor = overall_progress(&com_conversao, JobStage::Downloading, 1.0);
        assert!((valor - 70.0 / 83.0).abs() < 1e-9);
    }

    #[test]
    fn status_e_estagio_ida_e_volta() {
        for s in ["queued", "running", "done", "failed", "cancelled"] {
            assert_eq!(JobStatus::parse(s).unwrap().as_str(), s);
        }
        for s in [
            "waiting",
            "waiting_retry",
            "analyzing",
            "downloading",
            "converting",
            "moving",
            "done",
        ] {
            assert_eq!(JobStage::parse(s).unwrap().as_str(), s);
        }
        assert!(JobStatus::parse("x").is_none());
    }
}

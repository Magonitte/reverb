//! Tipos trocados com a UI: estado das ferramentas e eventos `tools://*` (arquitetura §15/§16).

use serde::Serialize;
use ts_rs::TS;

use super::spec::Tool;

pub const EVENT_PROGRESS: &str = "tools://progress";
pub const EVENT_CHANGED: &str = "tools://changed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ToolPhase {
    Downloading,
    Verifying,
    Extracting,
    /// Instalação das dependências do servidor (bgutil).
    InstallingDeps,
    Testing,
    /// Esperando os jobs em andamento soltarem as ferramentas.
    WaitingJobs,
}

/// Payload de `tools://progress`.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ToolProgress {
    pub tool: Tool,
    pub phase: ToolPhase,
    pub percent: u8,
}

/// Payload de `tools://changed`.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ToolChanged {
    pub tool: Tool,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ToolStatus {
    pub tool: Tool,
    /// Ferramenta indispensável para baixar (yt-dlp, FFmpeg).
    pub required: bool,
    pub installed: bool,
    pub version: Option<String>,
    pub previous_version: Option<String>,
    pub channel: Option<String>,
    pub installed_at: Option<String>,
    pub path: Option<String>,
    /// Última versão conhecida (da última verificação de atualizações).
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub last_checked: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateInfo {
    pub tool: Tool,
    pub current: Option<String>,
    pub latest: String,
    pub update_available: bool,
    pub checked_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "result")]
#[ts(export)]
pub enum InstallOutcome {
    /// Já estava na última versão (nada foi baixado).
    UpToDate { version: String },
    Installed {
        version: String,
        previous: Option<String>,
    },
}

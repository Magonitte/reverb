//! `tools/manifest.json`: versões instaladas de cada ferramenta (arquitetura §16).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::CoreResult;

pub const MANIFEST_FILE: &str = "manifest.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Installed {
    /// Rótulo da versão (nome da pasta em `tools/<ferramenta>/`).
    pub version: String,
    /// Versão reportada pelo próprio binário no teste de fumaça.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reported_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    pub installed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_updated_at: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolEntry {
    #[serde(default)]
    pub current: Option<Installed>,
    #[serde(default)]
    pub previous: Option<Installed>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub tools: BTreeMap<String, ToolEntry>,
}

impl Manifest {
    pub fn path(tools_dir: &Path) -> PathBuf {
        tools_dir.join(MANIFEST_FILE)
    }

    /// Manifesto ausente ou ilegível vira vazio (ferramentas serão reinstaladas).
    pub fn load(tools_dir: &Path) -> Self {
        let path = Self::path(tools_dir);
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                tracing::warn!(error = %e, "manifest.json ilegível; recomeçando vazio");
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Grava de forma atômica: arquivo temporário + renomeação.
    pub fn save(&self, tools_dir: &Path) -> CoreResult<()> {
        std::fs::create_dir_all(tools_dir)?;
        let path = Self::path(tools_dir);
        let tmp = tools_dir.join(format!("{MANIFEST_FILE}.tmp"));
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn entry(&self, tool: &str) -> Option<&ToolEntry> {
        self.tools.get(tool)
    }

    pub fn current(&self, tool: &str) -> Option<&Installed> {
        self.tools.get(tool).and_then(|e| e.current.as_ref())
    }
}

/// Data/hora UTC atual no formato `2026-10-01T12:34:56Z`.
pub fn now_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    rfc3339_from_secs(secs)
}

/// Segundos desde a época Unix ⇒ `YYYY-MM-DDTHH:MM:SSZ` (sem depender de crates de calendário).
pub fn rfc3339_from_secs(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Algoritmo "civil from days" (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests;

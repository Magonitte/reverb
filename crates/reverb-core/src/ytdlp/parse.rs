//! Parser das linhas `REVERB_PROGRESS` / `REVERB_DONE` do stdout do yt-dlp (arquitetura §8).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

pub const PROGRESS_PREFIX: &str = "REVERB_PROGRESS ";
pub const DONE_PREFIX: &str = "REVERB_DONE ";

/// Progresso do download. Sem `total` o progresso é indeterminado.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProgressUpdate {
    #[ts(type = "number")]
    pub downloaded: u64,
    #[ts(type = "number | null")]
    pub total: Option<u64>,
    /// Bytes por segundo.
    pub speed: Option<f64>,
    /// Segundos restantes.
    #[ts(type = "number | null")]
    pub eta: Option<u64>,
    /// `status == "finished"` do yt-dlp (o fim real do job é o `REVERB_DONE`).
    pub finished: bool,
}

/// Dados finais do download (`--print after_move:`).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DoneInfo {
    pub id: String,
    pub title: String,
    pub filepath: String,
    pub ext: String,
    pub abr: Option<f64>,
    pub acodec: Option<String>,
    pub format_id: Option<String>,
    pub duration: Option<f64>,
}

/// Formato da linha `REVERB_DONE` (chaves em snake_case, como o yt-dlp imprime).
#[derive(Deserialize)]
struct RawDone {
    id: String,
    title: String,
    filepath: String,
    ext: String,
    abr: Option<f64>,
    acodec: Option<String>,
    format_id: Option<String>,
    duration: Option<f64>,
}

fn whole_number(value: Option<&Value>) -> Option<u64> {
    let value = value?;
    value
        .as_u64()
        .or_else(|| value.as_f64().filter(|n| *n >= 0.0).map(|n| n as u64))
}

/// `Some` para uma linha `REVERB_PROGRESS` válida; qualquer outra coisa é ignorada.
pub fn parse_progress_line(line: &str) -> Option<ProgressUpdate> {
    let json = line.trim_end().strip_prefix(PROGRESS_PREFIX)?;
    let value: Value = serde_json::from_str(json).ok()?;
    let object = value.as_object()?;
    Some(ProgressUpdate {
        downloaded: whole_number(object.get("downloaded_bytes")).unwrap_or(0),
        total: whole_number(object.get("total_bytes"))
            .or_else(|| whole_number(object.get("total_bytes_estimate"))),
        speed: object.get("speed").and_then(Value::as_f64),
        eta: whole_number(object.get("eta")),
        finished: object.get("status").and_then(Value::as_str) == Some("finished"),
    })
}

/// `Some` para uma linha `REVERB_DONE` válida.
pub fn parse_done_line(line: &str) -> Option<DoneInfo> {
    let json = line.trim_end().strip_prefix(DONE_PREFIX)?;
    let raw: RawDone = serde_json::from_str(json).ok()?;
    Some(DoneInfo {
        id: raw.id,
        title: raw.title,
        filepath: raw.filepath,
        ext: raw.ext,
        abr: raw.abr,
        acodec: raw.acodec,
        format_id: raw.format_id,
        duration: raw.duration,
    })
}

#[cfg(test)]
mod tests;

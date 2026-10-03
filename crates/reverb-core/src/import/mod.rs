//! Collection imports retain source metadata and identify recordings by ISRC.
mod service;
mod source;
pub use service::*;
pub use source::*;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportedTrack {
    pub id: String,
    pub fields: crate::metadata::MetadataFields,
    pub duration_s: Option<f64>,
    pub isrc: Option<String>,
    pub explicit: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MatchResult {
    pub video_id: Option<String>,
    pub confidence: f64,
    pub via: crate::metadata::official::OfficialVia,
    pub bucket: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportedItem {
    pub track: ImportedTrack,
    pub matched: MatchResult,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportedCollection {
    pub provider: String,
    pub id: String,
    pub url: String,
    pub title: String,
    pub cover: Option<String>,
    pub tracks: Vec<ImportedTrack>,
    pub checksum: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportAnalysis {
    pub collection: ImportedCollection,
    pub items: Vec<ImportedItem>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportSelection {
    pub url: String,
    pub track_ids: Vec<String>,
    pub mode: String,
    pub profile_id: String,
    pub sync_options: Option<crate::sync::SyncCreate>,
}

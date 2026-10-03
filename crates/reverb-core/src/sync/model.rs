use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncCreate {
    pub url: String,
    pub title: Option<String>,
    pub profile_id: String,
    pub output_dir: Option<String>,
    pub interval_hours: u32,
    pub max_items: Option<u32>,
    pub remove_deleted: bool,
    pub write_m3u: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncUpdate {
    pub title: String,
    pub profile_id: String,
    pub output_dir: Option<String>,
    pub interval_hours: u32,
    pub max_items: Option<u32>,
    pub remove_deleted: bool,
    pub write_m3u: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct SyncResult {
    pub checksum: Option<String>,
    pub added: u32,
    pub removed: u32,
    pub failed: u32,
    pub duplicates: Vec<String>,
    pub unavailable: u32,
    pub running: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Sync {
    pub id: String,
    pub provider: String,
    pub url: String,
    pub playlist_id: Option<String>,
    pub title: String,
    pub thumbnail: Option<String>,
    pub profile_id: String,
    pub output_dir: Option<String>,
    pub interval_hours: u32,
    pub max_items: Option<u32>,
    pub remove_deleted: bool,
    pub write_m3u: bool,
    pub enabled: bool,
    #[ts(type = "number | null")]
    pub last_sync_at: Option<i64>,
    pub last_result: Option<SyncResult>,
    #[ts(type = "number")]
    pub created_at: i64,
    pub item_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncItem {
    pub sync_id: String,
    pub source_id: String,
    pub position: u32,
    pub title: Option<String>,
    pub state: String,
    pub job_id: Option<String>,
    #[ts(type = "number | null")]
    pub library_id: Option<i64>,
    pub job_status: Option<String>,
    pub file_path: Option<String>,
    pub missing: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlannedEntry {
    pub entry: crate::ytdlp::CollectionEntry,
    pub position: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SyncPlan {
    pub add: Vec<PlannedEntry>,
    pub removed: Vec<String>,
    pub reordered: Vec<PlannedEntry>,
    pub unchanged: Vec<PlannedEntry>,
    pub duplicates: Vec<String>,
    pub unavailable: u32,
}

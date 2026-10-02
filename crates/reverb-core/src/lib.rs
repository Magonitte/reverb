//! Núcleo do Reverb: Rust puro, sem dependência de Tauri (arquitetura §1).

pub mod app_info;
pub mod backend;
pub mod db;
pub mod error;
pub mod events;
pub mod exec;
pub mod logging;
pub mod metadata;
pub mod organize;
pub mod paths;
pub mod pipeline;
pub mod profiles;
pub mod queue;
pub mod settings;
pub mod tools;
pub mod transcode;
pub mod urlkind;
pub mod workspace;
pub mod ytdlp;

pub use app_info::AppInfo;
pub use db::Db;
pub use error::{CoreError, CoreResult};
pub use events::{EventSink, MemorySink};
pub use paths::DataPaths;
pub use settings::{Settings, SettingsPatch, SettingsService, SettingsView};
pub use tools::{Tool, ToolsConfig, ToolsManager};

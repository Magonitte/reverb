//! Núcleo do Reverb: Rust puro, sem dependência de Tauri (arquitetura §1).

pub mod app_info;
pub mod error;
pub mod events;
pub mod paths;

pub use app_info::AppInfo;
pub use error::{CoreError, CoreResult};
pub use events::{EventSink, MemorySink};
pub use paths::DataPaths;

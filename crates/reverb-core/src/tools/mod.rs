//! Gerenciador de ferramentas externas: yt-dlp, Deno, FFmpeg, fpcalc e bgutil (F02, arquitetura §16).

pub mod checksum;
pub mod github;
pub mod install;
pub mod js_runtime;
pub mod manager;
pub mod manifest;
pub mod pot;
pub mod process;
pub mod spec;
pub mod types;
pub mod version;

#[cfg(test)]
pub(crate) mod testutil;

pub use js_runtime::{detect_js_runtime, Detection, JsKind, JsRuntimeChoice};
pub use manager::{RunGuard, ToolsConfig, ToolsManager};
pub use pot::{pot_args, PotPolicy, PotServer, PotServerConfig};
pub use process::{kill_tree, run_capture, spawn_tool, tool_command, ToolOutput};
pub use spec::{Platform, Tool};
pub use types::{InstallOutcome, ToolChanged, ToolPhase, ToolProgress, ToolStatus, UpdateInfo};

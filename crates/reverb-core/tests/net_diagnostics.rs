//! F12 T12: real installed executables, runtime and FX1 simulation.
mod common;
use reverb_core::desktop::diagnostics::{self, DiagnosticLevel};
use reverb_core::{Db, MemorySink, SettingsService, Tool, ToolsConfig, ToolsManager};
use std::sync::Arc;

#[tokio::test]
#[ignore = "network"]
async fn t12_real_diagnostics_all_items_ok() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    let sink = Arc::new(MemorySink::new());
    let settings = Arc::new(
        SettingsService::new(db.clone(), sink.clone())
            .await
            .unwrap(),
    );
    settings
        .update(
            serde_json::from_value(serde_json::json!({"outputDir":dir.path().join("music")}))
                .unwrap(),
        )
        .await
        .unwrap();
    let tools = Arc::new(
        ToolsManager::new(
            ToolsConfig::new(common::tools_root()),
            db.clone(),
            settings.clone(),
            sink,
        )
        .unwrap(),
    );
    tools.update(Tool::Ytdlp).await.unwrap();
    let report = diagnostics::run(&db, settings, tools.clone())
        .await
        .unwrap();
    tools.shutdown().await;
    assert!(!report.items.is_empty());
    for item in &report.items {
        assert_eq!(
            item.level,
            DiagnosticLevel::Ok,
            "{}: {}",
            item.id,
            item.detail
        );
    }
    assert!(db.kv_get("diagnostics.last").await.unwrap().is_some());
}

//! F04 — T16 (rede): cancelar um download real em andamento pela fila. Roda com
//! `npm run verify:net`; usa diretórios temporários e as ferramentas de `.test-tools/`.

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use reverb_core::backend::{ToolsContext, YtDlpProcessBackend};
use reverb_core::queue::{
    EnqueueRequest, HealCoordinator, JobStatus, QueueDeps, QueueService, ToolsHeal, ToolsPipeline,
};
use reverb_core::ytdlp::YtDlpRunner;
use reverb_core::{Db, MemorySink, SettingsService, ToolsConfig, ToolsManager};
use sysinfo::{ProcessesToUpdate, System};

const FX2: &str = "https://music.youtube.com/watch?v=lYBUbBu4W08";

#[tokio::test]
#[ignore = "network"]
async fn t16_cancelar_download_real_nao_deixa_processos_nem_tmp() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("dados");
    let sink = Arc::new(MemorySink::new());
    let db = Db::open_in_memory().unwrap();
    let settings = Arc::new(
        SettingsService::new(db.clone(), sink.clone())
            .await
            .unwrap(),
    );
    settings
        .update(
            serde_json::from_value(serde_json::json!({
                "outputDir": temp.path().join("musicas"),
                // 0,2 MB/s: o download dura bem mais de 10 s e dá tempo de cancelar.
                "speedLimitMbps": 0.2,
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    let tools = Arc::new(
        ToolsManager::new(
            ToolsConfig::new(common::tools_root()),
            db.clone(),
            settings.clone(),
            sink.clone(),
        )
        .unwrap(),
    );
    let backend = Arc::new(YtDlpProcessBackend::new(
        YtDlpRunner::new(Some(tools.clone())),
        Arc::new(ToolsContext::new(tools.clone(), settings.clone())),
    ));
    let heal = Arc::new(HealCoordinator::new(
        Arc::new(ToolsHeal::new(tools.clone(), settings.clone())),
        db.clone(),
        sink.clone(),
    ));
    let queue = QueueService::start(QueueDeps {
        db,
        settings,
        sink: sink.clone(),
        runner: Arc::new(ToolsPipeline::new(backend, tools, data.clone())),
        heal,
        data_dir: data.clone(),
        start_paused: false,
    })
    .await
    .unwrap();

    let job = queue
        .enqueue(EnqueueRequest {
            url: FX2.to_string(),
            ..EnqueueRequest::default()
        })
        .await
        .unwrap();

    // Espera o 1º evento de progresso do download.
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let seen = sink
            .named("job://updated")
            .iter()
            .any(|e| e["id"] == job.id.as_str() && e["progress"].as_f64().is_some_and(|p| p > 0.0));
        if seen {
            break;
        }
        let current = queue.get(&job.id).await.unwrap().unwrap();
        assert!(
            current.status == JobStatus::Running || current.status == JobStatus::Queued,
            "o job terminou antes de cancelar: {current:?}"
        );
        assert!(Instant::now() < deadline, "sem progresso em 120 s");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    queue.cancel(&job.id).await.unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let current = queue.get(&job.id).await.unwrap().unwrap();
        if current.status == JobStatus::Cancelled {
            break;
        }
        assert!(Instant::now() < deadline, "não cancelou: {current:?}");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Nenhum yt-dlp/ffmpeg filho restante (procurados pela pasta tmp do job nos argumentos).
    tokio::time::sleep(Duration::from_secs(1)).await;
    let workspace = data.join("tmp").join(&job.id);
    let needle = workspace.to_string_lossy().to_lowercase();
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    let leftovers: Vec<String> = system
        .processes()
        .values()
        .filter(|p| {
            let line: String = p
                .cmd()
                .iter()
                .map(|a| a.to_string_lossy().to_lowercase())
                .collect::<Vec<_>>()
                .join(" ");
            line.contains(&needle)
        })
        .map(|p| p.name().to_string_lossy().into_owned())
        .collect();
    assert!(leftovers.is_empty(), "processos restantes: {leftovers:?}");
    assert!(!workspace.exists(), "tmp do job deve ser apagado");
    queue.shutdown().await;
}

//! Comandos `jobs list`, `queue add` e `queue run` (F04): a mesma fila persistente do app.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{bail, Result};
use reverb_core::backend::{ToolsContext, YtDlpProcessBackend};
use reverb_core::queue::{
    repo, EnqueueRequest, HealCoordinator, Job, JobStatus, QueueDeps, QueueService, ToolsHeal,
    ToolsPipeline,
};
use reverb_core::urlkind::{classify, UrlKind};
use reverb_core::ytdlp::YtDlpRunner;
use reverb_core::{Db, EventSink, SettingsService, ToolsManager};

/// Mostra no stderr as mudanças de estado dos jobs, a autocura e os avisos.
struct QueueSink {
    last: Mutex<std::collections::HashMap<String, String>>,
}

impl EventSink for QueueSink {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        match event {
            "job://updated" => {
                let id = payload["id"].as_str().unwrap_or("?").to_string();
                let status = payload["status"].as_str().unwrap_or("?");
                let stage = payload["stage"].as_str().unwrap_or("?");
                let line = format!("{status}/{stage}");
                let mut last = self.last.lock().expect("sink");
                if last.get(&id) != Some(&line) {
                    eprintln!("[job {}] {line}", &id[..id.len().min(8)]);
                    last.insert(id, line);
                }
            }
            "heal://state" => eprintln!("[autocura] {}", payload["stage"].as_str().unwrap_or("?")),
            "notice" => eprintln!("[aviso] {}", payload["i18nKey"].as_str().unwrap_or("?")),
            _ => {}
        }
    }
}

fn build(
    db: Db,
    manager: Arc<ToolsManager>,
    settings: Arc<SettingsService>,
    data_dir: PathBuf,
    start_paused: bool,
) -> QueueDeps {
    let sink: Arc<dyn EventSink> = Arc::new(QueueSink {
        last: Mutex::new(Default::default()),
    });
    let backend = Arc::new(YtDlpProcessBackend::new(
        YtDlpRunner::new(Some(Arc::clone(&manager))),
        Arc::new(ToolsContext::new(
            Arc::clone(&manager),
            Arc::clone(&settings),
        )),
    ));
    let heal = Arc::new(HealCoordinator::new(
        Arc::new(ToolsHeal::new(Arc::clone(&manager), Arc::clone(&settings))),
        db.clone(),
        Arc::clone(&sink),
    ));
    QueueDeps {
        db,
        settings,
        sink,
        runner: Arc::new(ToolsPipeline::new(backend, manager, data_dir.clone())),
        heal,
        data_dir,
        start_paused,
    }
}

fn minutes(seconds: f64) -> String {
    let total = seconds.round() as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

pub fn list(db: &Db, json: bool) -> Result<()> {
    let jobs = db.call_blocking(|conn| repo::list(conn))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&jobs)?);
        return Ok(());
    }
    for job in jobs {
        let name = job.title.as_deref().unwrap_or(&job.source_url);
        let duration = job.duration_s.map(minutes).unwrap_or_else(|| "-".into());
        println!(
            "{}\t{}\t{}\t{}x\t{duration}\t{name}",
            &job.id[..job.id.len().min(8)],
            job.status.as_str(),
            job.stage.as_str(),
            job.attempts,
        );
    }
    Ok(())
}

pub async fn add(
    db: Db,
    manager: Arc<ToolsManager>,
    settings: Arc<SettingsService>,
    data_dir: PathBuf,
    url: String,
    profile: Option<String>,
) -> Result<()> {
    let (url, source_id) = match classify(&url) {
        UrlKind::Video { url, source_id, .. } => (url, Some(source_id)),
        _ => bail!("a fila aceita apenas a URL de um vídeo: {url}"),
    };
    // Pausada: enfileirar não deve processar nada (o `queue run` faz isso).
    let queue = QueueService::start(build(db, manager, settings, data_dir, true)).await?;
    let job = queue
        .enqueue(EnqueueRequest {
            url,
            source_id,
            profile_id: profile,
            ..EnqueueRequest::default()
        })
        .await?;
    queue.shutdown().await;
    println!("{}", serde_json::to_string(&job)?);
    Ok(())
}

pub async fn run(
    db: Db,
    manager: Arc<ToolsManager>,
    settings: Arc<SettingsService>,
    data_dir: PathBuf,
    until_idle: bool,
) -> Result<()> {
    let queue = QueueService::start(build(db.clone(), manager, settings, data_dir, false)).await?;
    let waiter = queue.clone();
    let finished = tokio::select! {
        result = async move { if until_idle { waiter.wait_idle().await } else { std::future::pending().await } } => {
            result?;
            true
        }
        _ = tokio::signal::ctrl_c() => false,
    };
    queue.shutdown().await;
    let jobs: Vec<Job> = db.call_blocking(|conn| repo::list(conn))?;
    let failed = jobs
        .iter()
        .filter(|j| j.status == JobStatus::Failed)
        .count();
    println!(
        "{}",
        serde_json::json!({
            "done": jobs.iter().filter(|j| j.status == JobStatus::Done).count(),
            "failed": failed,
            "cancelled": jobs.iter().filter(|j| j.status == JobStatus::Cancelled).count(),
            "interrupted": !finished,
        })
    );
    if failed > 0 {
        bail!("{failed} job(s) falharam");
    }
    Ok(())
}

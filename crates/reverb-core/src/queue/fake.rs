//! `FakeBackend` (somente testes): um `DownloadBackend` com roteiro por URL, para exercitar a fila
//! sem rede, sem yt-dlp e com o tempo do Tokio pausado.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::backend::{DownloadBackend, JobSpec, ProgressCallback};
use crate::ytdlp::errors::{DownloadError, ErrorKind};
use crate::ytdlp::{Analysis, DoneInfo, ProgressUpdate, SearchResult, SearchSource};

#[derive(Clone)]
pub enum Script {
    /// Baixa em `ms` milissegundos emitindo `steps` progressos.
    Ok { ms: u64, steps: u32 },
    Fail {
        kind: ErrorKind,
        stderr: &'static str,
    },
    /// Falha `times` vezes e depois baixa.
    FailThenOk {
        kind: ErrorKind,
        times: u32,
        ms: u64,
        steps: u32,
    },
    /// Falha até `ok` virar `true` (o "conserto" feito pela autocura).
    FailUntil {
        kind: ErrorKind,
        stderr: &'static str,
        ok: Arc<AtomicBool>,
    },
    /// Nunca termina sozinho; só o cancelamento o interrompe.
    Hang,
}

impl Script {
    pub fn ok(ms: u64) -> Self {
        Self::Ok { ms, steps: 4 }
    }

    pub fn fail(kind: ErrorKind) -> Self {
        Self::Fail { kind, stderr: "" }
    }

    pub fn fail_then_ok(kind: ErrorKind, times: u32) -> Self {
        Self::FailThenOk {
            kind,
            times,
            ms: 100,
            steps: 2,
        }
    }
}

#[derive(Default)]
pub struct FakeBackend {
    scripts: Mutex<HashMap<String, Script>>,
    calls: Mutex<HashMap<String, Vec<Instant>>>,
    order: Mutex<Vec<String>>,
    running: AtomicUsize,
    max_running: AtomicUsize,
}

struct RunningGuard<'a>(&'a AtomicUsize);

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl FakeBackend {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn script(&self, url: &str, script: Script) {
        self.scripts.lock().unwrap().insert(url.to_string(), script);
    }

    /// Instantes (relógio do Tokio) em que `download(url)` foi chamado.
    pub fn calls(&self, url: &str) -> Vec<Instant> {
        self.calls
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .unwrap_or_default()
    }

    pub fn call_count(&self, url: &str) -> usize {
        self.calls(url).len()
    }

    /// URLs na ordem em que os downloads começaram.
    pub fn order(&self) -> Vec<String> {
        self.order.lock().unwrap().clone()
    }

    /// Maior número de downloads simultâneos observado.
    pub fn max_running(&self) -> usize {
        self.max_running.load(Ordering::SeqCst)
    }

    pub fn running(&self) -> usize {
        self.running.load(Ordering::SeqCst)
    }

    async fn play(
        &self,
        job: &JobSpec,
        ms: u64,
        steps: u32,
        on_progress: ProgressCallback<'_>,
        cancel: &CancellationToken,
    ) -> Result<DoneInfo, DownloadError> {
        let steps = steps.max(1);
        for step in 1..=steps {
            tokio::select! {
                () = tokio::time::sleep(Duration::from_millis(ms / u64::from(steps))) => {}
                () = cancel.cancelled() => return Err(DownloadError::cancelled()),
            }
            on_progress(ProgressUpdate {
                downloaded: u64::from(step) * 100,
                total: Some(u64::from(steps) * 100),
                speed: Some(1000.0),
                eta: Some(u64::from(steps - step)),
                finished: step == steps,
            });
        }
        let id = job
            .url
            .split("v=")
            .nth(1)
            .unwrap_or("faixa")
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect::<String>();
        let file = job.tmp_dir.join(format!("{id}.opus"));
        std::fs::write(&file, b"audio de mentira").map_err(|e| {
            DownloadError::new(ErrorKind::Disk, format!("não foi possível gravar: {e}"))
        })?;
        Ok(DoneInfo {
            id: id.clone(),
            title: format!("Faixa {id}"),
            filepath: file.to_string_lossy().into_owned(),
            ext: "opus".to_string(),
            abr: Some(128.0),
            acodec: Some("opus".to_string()),
            format_id: Some("251".to_string()),
            duration: Some(10.0),
        })
    }
}

fn failure(kind: ErrorKind, stderr: &str) -> DownloadError {
    let mut error = DownloadError::new(kind, format!("falha de teste: {}", kind.as_str()));
    if !stderr.is_empty() {
        error.stderr_tail = vec![stderr.to_string()];
    }
    error
}

#[async_trait]
impl DownloadBackend for FakeBackend {
    async fn analyze(
        &self,
        _url: &str,
        _cancel: &CancellationToken,
    ) -> Result<Analysis, DownloadError> {
        Err(DownloadError::new(
            ErrorKind::Unknown,
            "sem análise no fake",
        ))
    }

    async fn search(
        &self,
        _source: SearchSource,
        _query: &str,
        _limit: u32,
        _cancel: &CancellationToken,
    ) -> Result<Vec<SearchResult>, DownloadError> {
        Ok(Vec::new())
    }

    async fn download(
        &self,
        job: &JobSpec,
        on_progress: ProgressCallback<'_>,
        cancel: &CancellationToken,
    ) -> Result<DoneInfo, DownloadError> {
        self.order.lock().unwrap().push(job.url.clone());
        let attempt = {
            let mut calls = self.calls.lock().unwrap();
            let list = calls.entry(job.url.clone()).or_default();
            list.push(Instant::now());
            list.len() as u32
        };
        let now = self.running.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_running.fetch_max(now, Ordering::SeqCst);
        let _guard = RunningGuard(&self.running);

        let script = self
            .scripts
            .lock()
            .unwrap()
            .get(&job.url)
            .cloned()
            .unwrap_or(Script::Ok { ms: 100, steps: 2 });
        match script {
            Script::Ok { ms, steps } => self.play(job, ms, steps, on_progress, cancel).await,
            Script::Fail { kind, stderr } => Err(failure(kind, stderr)),
            Script::FailThenOk {
                kind,
                times,
                ms,
                steps,
            } => {
                if attempt <= times {
                    Err(failure(kind, ""))
                } else {
                    self.play(job, ms, steps, on_progress, cancel).await
                }
            }
            Script::FailUntil { kind, stderr, ok } => {
                if ok.load(Ordering::SeqCst) {
                    self.play(job, 100, 2, on_progress, cancel).await
                } else {
                    Err(failure(kind, stderr))
                }
            }
            Script::Hang => {
                cancel.cancelled().await;
                Err(DownloadError::cancelled())
            }
        }
    }
}

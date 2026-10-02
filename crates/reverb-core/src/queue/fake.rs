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
use crate::ytdlp::{Analysis, DoneInfo, ProgressUpdate, SearchResult, SearchSource, VideoInfo};

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
    audio: Mutex<Option<(Vec<u8>, bool)>>,
    source_abr: Mutex<Option<f64>>,
    scripts: Mutex<HashMap<String, Script>>,
    /// `analyze` por id de vídeo (F08).
    analyses: Mutex<HashMap<String, Analysis>>,
    /// `search` por `"<fonte>:<consulta>"` (F08).
    searches: Mutex<HashMap<String, Vec<SearchResult>>>,
    search_log: Mutex<Vec<String>>,
    analyze_log: Mutex<Vec<String>>,
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
    pub fn set_collection(&self, url: &str, info: crate::ytdlp::CollectionInfo) {
        self.analyses
            .lock()
            .unwrap()
            .insert(url.to_owned(), Analysis::Collection { info });
    }
    /// Opus válido para os testes de pós-processamento; padrão antigo continua sem FFmpeg.
    pub fn set_source_abr(&self, abr: f64) {
        *self.source_abr.lock().unwrap() = Some(abr);
    }
    pub fn set_audio(&self, bytes: Vec<u8>, readonly: bool) {
        *self.audio.lock().unwrap() = Some((bytes, readonly));
    }
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn script(&self, url: &str, script: Script) {
        self.scripts.lock().unwrap().insert(url.to_string(), script);
    }

    /// `analyze` de qualquer URL com esse id de vídeo devolve `info`.
    pub fn set_video(&self, info: VideoInfo) {
        self.analyses.lock().unwrap().insert(
            info.id.clone(),
            Analysis::Video {
                info: Box::new(info),
            },
        );
    }

    pub fn set_search(&self, source: SearchSource, query: &str, results: Vec<SearchResult>) {
        self.searches
            .lock()
            .unwrap()
            .insert(search_key(source, query), results);
    }

    /// Buscas feitas, como `"<fonte>:<consulta>"`.
    pub fn search_log(&self) -> Vec<String> {
        self.search_log.lock().unwrap().clone()
    }

    /// URLs analisadas, na ordem das chamadas.
    pub fn analyze_log(&self) -> Vec<String> {
        self.analyze_log.lock().unwrap().clone()
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
        let audio = self.audio.lock().unwrap().clone();
        let bytes = audio
            .as_ref()
            .map_or(b"audio de mentira".as_slice(), |(bytes, _)| {
                bytes.as_slice()
            });
        std::fs::write(&file, bytes).map_err(|e| {
            DownloadError::new(ErrorKind::Disk, format!("não foi possível gravar: {e}"))
        })?;
        if audio.is_some_and(|(_, readonly)| readonly) {
            let mut permissions = std::fs::metadata(&file).unwrap().permissions();
            permissions.set_readonly(true);
            std::fs::set_permissions(&file, permissions).unwrap();
        }
        // Vídeo registrado com `set_video`: o título e a duração dele (F08).
        let known =
            video_id(&job.url).and_then(|vid| match self.analyses.lock().unwrap().get(&vid) {
                Some(Analysis::Video { info }) => Some((info.title.clone(), info.duration)),
                _ => None,
            });
        let (title, duration) = known.unwrap_or_else(|| (format!("Faixa {id}"), Some(10.0)));
        Ok(DoneInfo {
            id: id.clone(),
            title,
            filepath: file.to_string_lossy().into_owned(),
            ext: "opus".to_string(),
            abr: Some(self.source_abr.lock().unwrap().unwrap_or(128.0)),
            acodec: Some("opus".to_string()),
            format_id: Some("251".to_string()),
            duration,
        })
    }
}

fn search_key(source: SearchSource, query: &str) -> String {
    let source = match source {
        SearchSource::YtMusic => "ytmusic",
        SearchSource::Youtube => "youtube",
    };
    format!("{source}:{query}")
}

/// Id do vídeo numa URL do YouTube (`v=ID` ou `youtu.be/ID`).
fn video_id(url: &str) -> Option<String> {
    match crate::urlkind::classify(url) {
        crate::urlkind::UrlKind::Video { source_id, .. } => Some(source_id),
        _ => None,
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
        url: &str,
        _cancel: &CancellationToken,
    ) -> Result<Analysis, DownloadError> {
        self.analyze_log.lock().unwrap().push(url.to_string());
        if let Some(analysis) = self.analyses.lock().unwrap().get(url).cloned() {
            return Ok(analysis);
        }
        video_id(url)
            .and_then(|id| self.analyses.lock().unwrap().get(&id).cloned())
            .ok_or_else(|| DownloadError::new(ErrorKind::Unknown, "sem análise no fake"))
    }

    async fn search(
        &self,
        source: SearchSource,
        query: &str,
        limit: u32,
        _cancel: &CancellationToken,
    ) -> Result<Vec<SearchResult>, DownloadError> {
        let key = search_key(source, query);
        self.search_log.lock().unwrap().push(key.clone());
        let mut found = self
            .searches
            .lock()
            .unwrap()
            .get(&key)
            .cloned()
            .unwrap_or_default();
        found.truncate(limit as usize);
        Ok(found)
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

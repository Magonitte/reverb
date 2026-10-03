//! `QueueService`: o laço que inicia jobs até o limite dinâmico de `parallelism`, com pausa,
//! cancelamento, retry com backoff, autocura e eventos com throttle (arquitetura §9 e §10).
//!
//! Regras de concorrência: o `Mutex` do estado (`std`) nunca atravessa um `.await`; o `ops`
//! (`tokio`) serializa só as operações curtas que mexem na fila e no banco, nunca o download.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Notify;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use super::heal::{HealCoordinator, HealOutcome};
use super::model::{
    overall_progress, DuplicateHit, EnqueueRequest, Job, JobStage, JobStatus, MoveTarget,
    QueueState,
};
use super::repo;
use super::runner::PipelineRunner;
use crate::db::Db;
use crate::error::{CoreError, CoreResult};
use crate::events::EventSink;
use crate::paths::resolve_output_dir;
use crate::pipeline::{PipelineEvent, PipelineJob, PipelineOutput};
use crate::profiles::profile;
use crate::settings::SettingsService;
use crate::workspace::sweep_orphans;
use crate::ytdlp::errors::{DownloadError, ErrorKind};

pub const EVENT_JOB_UPDATED: &str = "job://updated";
pub const EVENT_JOB_REMOVED: &str = "job://removed";
pub const EVENT_QUEUE_STATE: &str = "queue://state";

/// Intervalo mínimo entre eventos `job://updated` de progresso de um mesmo job.
const THROTTLE: Duration = Duration::from_millis(250);
/// Esperas entre tentativas (§9); depois da última, repete 120 s.
const BACKOFF_SECS: [u64; 3] = [5, 30, 120];
const PROVIDER: &str = "youtube";

pub struct QueueDeps {
    pub db: Db,
    pub settings: Arc<SettingsService>,
    pub sink: Arc<dyn EventSink>,
    pub runner: Arc<dyn PipelineRunner>,
    pub heal: Arc<HealCoordinator>,
    pub data_dir: PathBuf,
    /// Começa pausada (o CLI enfileira sem processar).
    pub start_paused: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StopReason {
    User,
    Remove,
    Shutdown,
}

struct Live {
    cancel: CancellationToken,
    reason: Option<StopReason>,
    job: Job,
    present: Vec<JobStage>,
    started_epoch: u64,
    last_emit: Option<Instant>,
}

#[derive(Default)]
struct State {
    paused: bool,
    healing: u32,
    running: HashMap<String, Live>,
    /// Jobs `queued` esperando o fim do backoff (não podem iniciar ainda).
    waiting: HashSet<String>,
    last_playlist_start: Option<Instant>,
}

struct Inner {
    db: Db,
    settings: Arc<SettingsService>,
    sink: Arc<dyn EventSink>,
    runner: Arc<dyn PipelineRunner>,
    heal: Arc<HealCoordinator>,
    state: Mutex<State>,
    ops: tokio::sync::Mutex<()>,
    wake: Notify,
    idle: Notify,
    stop: CancellationToken,
}

#[derive(Clone)]
pub struct QueueService {
    inner: Arc<Inner>,
}

fn retry_delay(attempts: u32) -> Duration {
    let index = (attempts.max(1) as usize - 1).min(BACKOFF_SECS.len() - 1);
    Duration::from_secs(BACKOFF_SECS[index])
}

impl QueueService {
    /// Restaura a fila do banco (`running ⇒ queued`), limpa `tmp/` órfão e inicia o laço.
    pub async fn start(deps: QueueDeps) -> CoreResult<Self> {
        let restored = deps.db.call(|conn| repo::restore(conn)).await?;
        if restored > 0 {
            tracing::info!(restored, "jobs interrompidos voltaram para a fila");
        }
        sweep_orphans(&deps.data_dir, Duration::ZERO);
        let inner = Arc::new(Inner {
            db: deps.db,
            settings: deps.settings,
            sink: deps.sink,
            runner: deps.runner,
            heal: deps.heal,
            state: Mutex::new(State {
                paused: deps.start_paused,
                ..State::default()
            }),
            ops: tokio::sync::Mutex::new(()),
            wake: Notify::new(),
            idle: Notify::new(),
            stop: CancellationToken::new(),
        });
        tokio::spawn(run_loop(Arc::clone(&inner)));
        inner.wake.notify_one();
        Ok(Self { inner })
    }

    /// Reavalia a fila (ex.: o usuário mudou `parallelism`).
    pub fn wake(&self) {
        self.inner.wake.notify_one();
    }

    pub async fn enqueue(&self, mut request: EnqueueRequest) -> CoreResult<Job> {
        let inner = &self.inner;
        if let Some(id) = request.options.as_ref().and_then(|o| o.upgrade_library_id) {
            let item = inner
                .db
                .call(move |conn| crate::library::get(conn, id))
                .await?
                .ok_or_else(|| CoreError::invalid("Upgrade item missing"))?;
            if item.provider.as_deref() != Some("youtube") || item.missing {
                return Err(CoreError::invalid("Upgrade item unavailable"));
            }
            request.url = item
                .source_url
                .ok_or_else(|| CoreError::invalid("Upgrade source missing"))?;
            request.source_id = item.source_id;
            request.profile_id = item.profile_id;
            request.allow_duplicate = true;
        }
        let url = request.url.trim().to_string();
        if url.is_empty() {
            return Err(CoreError::invalid("a URL é obrigatória"));
        }
        let settings = inner.settings.get();
        let profile_id = request
            .profile_id
            .clone()
            .unwrap_or_else(|| settings.default_profile.clone());
        if profile(&profile_id).is_none() {
            return Err(CoreError::invalid(format!(
                "perfil desconhecido: {profile_id}"
            )));
        }
        let queue_limit = settings.queue_limit;
        let _ops = inner.ops.lock().await;
        let now = repo::now();
        let kind = if request
            .options
            .as_ref()
            .is_some_and(|o| o.upgrade_library_id.is_some())
        {
            "upgrade"
        } else if request
            .options
            .as_ref()
            .is_some_and(|o| o.split_chapters == Some(true))
        {
            "chapters"
        } else if request.playlist_ctx.is_some() {
            "playlist_item"
        } else {
            "single"
        };
        let sync_id = request
            .playlist_ctx
            .as_ref()
            .and_then(|ctx| ctx.sync_id.clone());
        let mut job = Job {
            id: uuid_v4(),
            kind: kind.to_string(),
            provider: crate::sources::provider_id(&url).to_string(),
            source_url: url,
            source_id: request.source_id.clone(),
            title: request.title.clone(),
            artist: None,
            thumbnail: request.thumbnail.clone(),
            duration_s: request.duration_s,
            profile_id: profile_id.clone(),
            options: request.options.clone().unwrap_or_default(),
            metadata_override: request.metadata_override.clone(),
            confidence: None,
            metadata_result: None,
            warnings: Vec::new(),
            playlist_ctx: request.playlist_ctx.clone(),
            sync_id,
            status: JobStatus::Queued,
            stage: JobStage::Waiting,
            progress: 0.0,
            overall_progress: 0.0,
            speed_bps: None,
            eta_s: None,
            error_kind: None,
            error_message: None,
            attempts: 0,
            output_path: None,
            library_id: None,
            position: 0,
            created_at: now,
            updated_at: now,
            finished_at: None,
        };
        let allow_duplicate = request.allow_duplicate;
        let priority = request.priority;
        let job = inner
            .db
            .call(move |conn| {
                if let Some(id)=job.options.upgrade_library_id {
                    let exists:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM jobs WHERE kind='upgrade' AND status IN ('queued','running') AND json_extract(options_json,'$.upgradeLibraryId')=?)",[id],|r|r.get(0))?;
                    if exists{return Err(CoreError::coded("duplicate","Upgrade already queued"));}
                }
                if repo::count_active(conn)? >= queue_limit {
                    return Err(CoreError::coded(
                        "queue_full",
                        format!("a fila está cheia ({queue_limit} jobs)"),
                    ));
                }
                if !allow_duplicate {
                    if let Some(value)=&job.metadata_override {
                        let ids=crate::library::find_by_fingerprint(conn,value["acoustidId"].as_str(),value["mbRecordingId"].as_str())?;
                        if !ids.is_empty(){return Err(CoreError::coded("duplicate","Recording already in library"));}
                    }
                    if let Some(source_id) = &job.source_id {
                        let hits = repo::find_duplicates(
                            conn,
                            &job.provider,
                            std::slice::from_ref(source_id),
                            &job.profile_id,
                        )?;
                        if !hits.is_empty() {
                            return Err(CoreError::coded(
                                "duplicate",
                                format!("já existe: {source_id}"),
                            ));
                        }
                    }
                }
                job.position = if priority {
                    repo::front_position(conn)?
                } else {
                    repo::next_position(conn)?
                };
                repo::insert(conn, &job)?;
                Ok(job)
            })
            .await?;
        inner.emit_job(&job);
        inner.emit_state().await;
        inner.wake.notify_one();
        Ok(job)
    }

    pub async fn check_duplicates(
        &self,
        source_ids: Vec<String>,
        profile_id: Option<String>,
    ) -> CoreResult<Vec<DuplicateHit>> {
        let profile_id = profile_id.unwrap_or_else(|| self.inner.settings.get().default_profile);
        self.inner
            .db
            .call(move |conn| {
                let mut hits = Vec::new();
                for provider in [PROVIDER, "soundcloud", "bandcamp", "archive", "jamendo"] {
                    hits.extend(repo::find_duplicates(
                        conn,
                        provider,
                        &source_ids,
                        &profile_id,
                    )?);
                }
                Ok(hits)
            })
            .await
    }

    /// Todos os jobs na ordem da fila; os em execução trazem o progresso ao vivo.
    pub async fn list(&self) -> CoreResult<Vec<Job>> {
        let mut jobs = self.inner.db.call(|conn| repo::list(conn)).await?;
        let state = self.inner.state.lock().expect("estado da fila");
        for job in &mut jobs {
            if let Some(live) = state.running.get(&job.id) {
                *job = live.job.clone();
            }
        }
        Ok(jobs)
    }

    pub async fn get(&self, id: &str) -> CoreResult<Option<Job>> {
        {
            let state = self.inner.state.lock().expect("estado da fila");
            if let Some(live) = state.running.get(id) {
                return Ok(Some(live.job.clone()));
            }
        }
        let id = id.to_string();
        self.inner.db.call(move |conn| repo::get(conn, &id)).await
    }

    /// Cancela um job em fila (vira `cancelled` na hora) ou em execução (mata o processo).
    pub async fn cancel(&self, id: &str) -> CoreResult<()> {
        let _ops = self.inner.ops.lock().await;
        self.inner.cancel_locked(id).await
    }

    pub async fn cancel_all(&self) -> CoreResult<()> {
        let _ops = self.inner.ops.lock().await;
        let jobs = self.inner.db.call(|conn| repo::list(conn)).await?;
        for job in jobs {
            if matches!(job.status, JobStatus::Queued | JobStatus::Running) {
                self.inner.cancel_locked(&job.id).await?;
            }
        }
        Ok(())
    }

    /// `failed|cancelled ⇒ queued`, com as tentativas zeradas, no fim da fila.
    pub async fn retry(&self, id: &str) -> CoreResult<Job> {
        let _ops = self.inner.ops.lock().await;
        let owned = id.to_string();
        let job = self
            .inner
            .db
            .call(move |conn| {
                let mut job = repo::get(conn, &owned)?
                    .ok_or_else(|| CoreError::invalid(format!("job não encontrado: {owned}")))?;
                if !matches!(job.status, JobStatus::Failed | JobStatus::Cancelled) {
                    return Err(CoreError::invalid(
                        "só é possível tentar de novo um job que falhou ou foi cancelado",
                    ));
                }
                job.status = JobStatus::Queued;
                job.stage = JobStage::Waiting;
                job.attempts = 0;
                job.progress = 0.0;
                job.overall_progress = 0.0;
                job.speed_bps = None;
                job.eta_s = None;
                job.error_kind = None;
                job.error_message = None;
                job.finished_at = None;
                job.position = repo::next_position(conn)?;
                job.updated_at = repo::now();
                repo::save(conn, &job)?;
                Ok(job)
            })
            .await?;
        self.inner.emit_job(&job);
        self.inner.emit_state().await;
        self.inner.wake.notify_one();
        Ok(job)
    }

    /// Remove o job da lista (se estiver rodando, cancela e remove ao terminar).
    pub async fn remove(&self, id: &str) -> CoreResult<()> {
        let _ops = self.inner.ops.lock().await;
        {
            let mut state = self.inner.state.lock().expect("estado da fila");
            if let Some(live) = state.running.get_mut(id) {
                live.reason = Some(StopReason::Remove);
                live.cancel.cancel();
                return Ok(());
            }
            state.waiting.remove(id);
        }
        let owned = id.to_string();
        if self
            .inner
            .db
            .call(move |conn| repo::delete(conn, &owned))
            .await?
        {
            self.inner.emit_removed(id);
            self.inner.emit_state().await;
        }
        Ok(())
    }

    pub async fn clear_finished(&self) -> CoreResult<u32> {
        let _ops = self.inner.ops.lock().await;
        let ids = self
            .inner
            .db
            .call(|conn| repo::delete_finished(conn))
            .await?;
        for id in &ids {
            self.inner.emit_removed(id);
        }
        Ok(ids.len() as u32)
    }

    pub async fn move_job(&self, id: &str, target: MoveTarget) -> CoreResult<()> {
        let _ops = self.inner.ops.lock().await;
        let owned = id.to_string();
        let changed = self
            .inner
            .db
            .call(move |conn| repo::move_job(conn, &owned, &target))
            .await?;
        for job in &changed {
            self.inner.emit_job(job);
        }
        Ok(())
    }

    pub async fn pause(&self) {
        self.inner.state.lock().expect("estado da fila").paused = true;
        self.inner.emit_state().await;
    }

    pub async fn resume(&self) {
        self.inner.state.lock().expect("estado da fila").paused = false;
        self.inner.emit_state().await;
        self.inner.wake.notify_one();
    }

    pub async fn state(&self) -> CoreResult<QueueState> {
        self.inner.queue_state().await
    }

    /// Espera a fila esvaziar (nada em fila, nada rodando). Usado por `queue run --until-idle`.
    pub async fn wait_idle(&self) -> CoreResult<()> {
        loop {
            let notified = self.inner.idle.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.inner.is_idle().await? {
                return Ok(());
            }
            notified.await;
        }
    }

    /// Para o laço e interrompe os jobs em execução: eles voltam para `queued` no banco.
    pub async fn shutdown(&self) {
        self.inner.stop.cancel();
        {
            let mut state = self.inner.state.lock().expect("estado da fila");
            for live in state.running.values_mut() {
                live.reason.get_or_insert(StopReason::Shutdown);
                live.cancel.cancel();
            }
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if self
                .inner
                .state
                .lock()
                .expect("estado da fila")
                .running
                .is_empty()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

fn uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl Inner {
    fn emit_job(&self, job: &Job) {
        if let Ok(payload) = serde_json::to_value(job) {
            self.sink.emit(EVENT_JOB_UPDATED, payload);
        }
    }

    fn emit_removed(&self, id: &str) {
        self.sink
            .emit(EVENT_JOB_REMOVED, serde_json::json!({ "id": id }));
    }

    async fn queue_state(&self) -> CoreResult<QueueState> {
        let queued = self
            .db
            .call(|conn| repo::count_status(conn, JobStatus::Queued))
            .await?;
        let state = self.state.lock().expect("estado da fila");
        Ok(QueueState {
            paused: state.paused,
            running: state.running.len() as u32,
            queued,
            healing: state.healing > 0 || self.heal.is_active(),
        })
    }

    async fn emit_state(&self) {
        if let Ok(state) = self.queue_state().await {
            if let Ok(payload) = serde_json::to_value(state) {
                self.sink.emit(EVENT_QUEUE_STATE, payload);
            }
        }
        self.idle.notify_waiters();
    }

    async fn is_idle(&self) -> CoreResult<bool> {
        let queued = self
            .db
            .call(|conn| repo::count_status(conn, JobStatus::Queued))
            .await?;
        Ok(queued == 0
            && self
                .state
                .lock()
                .expect("estado da fila")
                .running
                .is_empty())
    }

    async fn persist(&self, job: &Job) {
        let job = job.clone();
        if let Err(error) = self.db.call(move |conn| repo::save(conn, &job)).await {
            tracing::error!(%error, "não foi possível gravar o job");
        }
    }

    /// Chamado com o `ops` já seguro pelo chamador.
    async fn cancel_locked(&self, id: &str) -> CoreResult<()> {
        {
            let mut state = self.state.lock().expect("estado da fila");
            if let Some(live) = state.running.get_mut(id) {
                live.reason.get_or_insert(StopReason::User);
                live.cancel.cancel();
                return Ok(());
            }
            state.waiting.remove(id);
        }
        let owned = id.to_string();
        let cancelled = self
            .db
            .call(move |conn| {
                let Some(mut job) = repo::get(conn, &owned)? else {
                    return Ok(None);
                };
                if job.status != JobStatus::Queued {
                    return Ok(None);
                }
                job.status = JobStatus::Cancelled;
                job.stage = JobStage::Waiting;
                job.updated_at = repo::now();
                job.finished_at = Some(job.updated_at);
                repo::save(conn, &job)?;
                Ok(Some(job))
            })
            .await?;
        if let Some(job) = cancelled {
            self.emit_job(&job);
            self.emit_state().await;
        }
        Ok(())
    }

    /// Inicia jobs enquanto houver vaga, a fila não estiver pausada e nenhuma autocura rodando.
    async fn start_ready(self: &Arc<Self>) {
        loop {
            let _ops = self.ops.lock().await;
            let limit = self.settings.get().parallelism as usize;
            let (mut skip, pacing_delay) = {
                let state = self.state.lock().expect("estado da fila");
                if state.paused
                    || state.healing > 0
                    || self.heal.is_active()
                    || state.running.len() >= limit
                {
                    return;
                }
                let delay = state.last_playlist_start.and_then(|last| {
                    (last
                        + Duration::from_secs(u64::from(
                            self.settings.get().playlist_pacing_seconds,
                        )))
                    .checked_duration_since(Instant::now())
                    .filter(|delay| !delay.is_zero())
                });
                (state.waiting.clone(), delay)
            };
            let claimed = self
                .db
                .call(move |conn| loop {
                    let Some(next) = repo::next_queued(conn, &skip)? else {
                        return Ok(None);
                    };
                    if pacing_delay.is_some()
                        && matches!(next.kind.as_str(), "playlist_item" | "upgrade")
                    {
                        skip.insert(next.id);
                        continue;
                    }
                    return repo::claim(conn, &next.id);
                })
                .await;
            let job = match claimed {
                Ok(Some(job)) => job,
                Ok(None) => {
                    if let Some(delay) = pacing_delay {
                        let inner = Arc::clone(self);
                        tokio::spawn(async move {
                            tokio::select! { _=inner.stop.cancelled()=>{}, _=tokio::time::sleep(delay)=>inner.wake.notify_one() }
                        });
                    }
                    return;
                }
                Err(error) => {
                    tracing::error!(%error, "falha ao reservar o próximo job");
                    return;
                }
            };
            let cancel = CancellationToken::new();
            let started_epoch = self.heal.epoch();
            let present = planned_stages(&job.profile_id);
            if matches!(job.kind.as_str(), "playlist_item" | "upgrade") {
                self.state
                    .lock()
                    .expect("estado da fila")
                    .last_playlist_start = Some(Instant::now());
            }
            self.state.lock().expect("estado da fila").running.insert(
                job.id.clone(),
                Live {
                    cancel: cancel.clone(),
                    reason: None,
                    job: job.clone(),
                    present,
                    started_epoch,
                    last_emit: Some(Instant::now()),
                },
            );
            self.emit_job(&job);
            drop(_ops);
            self.emit_state().await;
            tokio::spawn(run_job(Arc::clone(self), job, cancel));
        }
    }

    fn on_event(&self, id: &str, event: PipelineEvent) {
        let snapshot = {
            let mut state = self.state.lock().expect("estado da fila");
            let Some(live) = state.running.get_mut(id) else {
                return;
            };
            let previous = live.job.stage;
            let (stage, fraction) = match event {
                PipelineEvent::Download(update) => {
                    live.job.speed_bps = update.speed;
                    live.job.eta_s = update.eta.map(|e| e as i64);
                    let fraction = if update.finished {
                        1.0
                    } else {
                        update
                            .total
                            .filter(|t| *t > 0)
                            .map_or(0.0, |t| update.downloaded as f64 / t as f64)
                    };
                    (JobStage::Downloading, fraction)
                }
                PipelineEvent::Convert(percent) => {
                    live.job.speed_bps = None;
                    live.job.eta_s = None;
                    insert_before_moving(&mut live.present, JobStage::Converting);
                    (JobStage::Converting, f64::from(percent) / 100.0)
                }
                PipelineEvent::Analyzing => (JobStage::Analyzing, 0.0),
                PipelineEvent::SourceSwitched { url, source_id } => {
                    live.job.source_url = url;
                    live.job.source_id = Some(source_id);
                    (previous, live.job.progress)
                }
                PipelineEvent::Identifying => {
                    live.job.speed_bps = None;
                    live.job.eta_s = None;
                    insert_before_moving(&mut live.present, JobStage::Metadata);
                    (JobStage::Metadata, 0.0)
                }
                PipelineEvent::Identified(result) => {
                    live.job.apply_metadata(&result);
                    (JobStage::Metadata, 1.0)
                }
                PipelineEvent::Stage(stage) => {
                    live.job.speed_bps = None;
                    live.job.eta_s = None;
                    insert_before_moving(&mut live.present, stage);
                    (stage, 0.0)
                }
                PipelineEvent::Warning(warning) => {
                    if !live.job.warnings.contains(&warning) {
                        live.job.warnings.push(warning);
                    }
                    (previous, live.job.progress)
                }
            };
            live.job.stage = stage;
            live.job.progress = fraction.clamp(0.0, 1.0);
            let overall = overall_progress(&live.present, stage, fraction);
            live.job.overall_progress = live.job.overall_progress.max(overall);
            live.job.updated_at = repo::now();
            let now = Instant::now();
            let due = stage != previous
                || live
                    .last_emit
                    .is_none_or(|last| now.duration_since(last) >= THROTTLE);
            if due {
                live.last_emit = Some(now);
                Some(live.job.clone())
            } else {
                None
            }
        };
        if let Some(job) = snapshot {
            self.emit_job(&job);
        }
    }

    fn take_live(&self, id: &str) -> Option<Live> {
        self.state
            .lock()
            .expect("estado da fila")
            .running
            .remove(id)
    }

    fn reason_of(&self, id: &str) -> Option<StopReason> {
        self.state
            .lock()
            .expect("estado da fila")
            .running
            .get(id)
            .and_then(|live| live.reason)
    }

    async fn finish_ok(self: &Arc<Self>, id: &str, mut output: PipelineOutput) {
        if self.reason_of(id).is_some() {
            self.finish_err(id, DownloadError::cancelled()).await;
            return;
        }
        let Some(mut job) = self
            .state
            .lock()
            .expect("estado da fila")
            .running
            .get(id)
            .map(|live| live.job.clone())
        else {
            return;
        };
        let now = repo::now();
        job.status = JobStatus::Done;
        job.stage = JobStage::Done;
        job.progress = 1.0;
        job.overall_progress = 1.0;
        job.speed_bps = None;
        job.eta_s = None;
        job.error_kind = None;
        job.error_message = None;
        job.output_path = Some(output.path.to_string_lossy().into_owned());
        if let Some(file) = &output.library_file {
            job.title = Some(file.tags.title.clone());
            job.artist = file.tags.artist.clone();
            job.duration_s = file.probe.as_ref().map(|probe| probe.duration_s);
            job.source_id = Some(output.done.id.clone());
        }
        if job.title.is_none() {
            job.title = Some(output.done.title.clone());
        }
        if job.duration_s.is_none() {
            job.duration_s = output.done.duration;
        }
        job.updated_at = now;
        job.finished_at = Some(now);
        if !output.chapter_files.is_empty() {
            job.kind = "chapters".into();
        }
        let mut recorded = job.clone();
        let file = output.library_file.take();
        let chapters = std::mem::take(&mut output.chapter_files);
        let upgrade_id = output.upgrade_id;
        let saved = self
            .db
            .call(move |conn| {
                let tx = conn.transaction()?;
                if let Some(file) = file {
                    recorded.library_id=if let Some(id)=upgrade_id {
                        crate::library::files::update_tags(&tx,&file.file_path,&file.tags)?;
                        tx.execute("UPDATE library SET source_abr_kbps=?,bitrate_kbps=?,duration_s=?,codec=?,replaygain_db=?,updated_at=unixepoch() WHERE id=?",rusqlite::params![file.source_abr_kbps,file.probe.as_ref().and_then(|p|p.bitrate_kbps),file.probe.as_ref().map(|p|p.duration_s),file.probe.as_ref().map(|p|&p.codec),file.replaygain_db,id])?;Some(id)
                    }else{Some(crate::library::insert_from_job(&tx,&recorded,&file)?)};
                }
                for chapter in chapters {
                    crate::library::insert_from_job(&tx, &recorded, &chapter)?;
                }
                repo::save(&tx, &recorded)?;
                if let Some(library_id) = recorded.library_id {
                    tx.execute(
                        "UPDATE sync_items SET library_id=? WHERE job_id=?",
                        rusqlite::params![library_id, recorded.id],
                    )?;
                }
                tx.commit()?;
                Ok(recorded)
            })
            .await;
        let job = match saved {
            Ok(job) => job,
            Err(error) => {
                self.finish_err(id, DownloadError::new(ErrorKind::Disk, error.to_string()))
                    .await;
                return;
            }
        };
        self.take_live(id);
        if let Some(publication) = &mut output.publication {
            publication.commit();
        }
        if let Some(replacement) = &mut output.replacement {
            if let Err(error) = replacement.commit() {
                tracing::warn!(kind = error.kind(), "Original audio retained after upgrade");
            }
        }
        for publication in &mut output.chapter_publications {
            publication.commit();
        }
        if let Some(library_id) = job.library_id {
            self.sink.emit(
                "library://changed",
                serde_json::json!({ "ids": [library_id] }),
            );
        }
        self.emit_job(&job);
        self.after_finish().await;
    }

    async fn after_finish(&self) {
        self.emit_state().await;
        self.wake.notify_one();
    }

    async fn finish_err(self: &Arc<Self>, id: &str, error: DownloadError) {
        // Falhas que a autocura trata: a decisão sai antes de liberar a vaga do job.
        let mut heal_outcome = None;
        if matches!(error.kind, ErrorKind::Extractor | ErrorKind::BotCheck)
            && self.reason_of(id).is_none()
        {
            let started_epoch = self
                .state
                .lock()
                .expect("estado da fila")
                .running
                .get(id)
                .map_or(0, |live| live.started_epoch);
            self.state.lock().expect("estado da fila").healing += 1;
            let text = format!("{}\n{}", error.message, error.stderr_tail.join("\n"));
            heal_outcome = Some(self.heal.handle(error.kind, &text, started_epoch).await);
            self.state.lock().expect("estado da fila").healing -= 1;
        }

        let Some(live) = self.take_live(id) else {
            return;
        };
        let mut job = live.job;
        let now = repo::now();
        job.speed_bps = None;
        job.eta_s = None;
        job.updated_at = now;

        let reason = live.reason.or(if error.kind == ErrorKind::Cancelled {
            Some(StopReason::User)
        } else {
            None
        });
        match reason {
            Some(StopReason::Remove) => {
                let owned = id.to_string();
                let _ = self.db.call(move |conn| repo::delete(conn, &owned)).await;
                self.emit_removed(id);
                self.after_finish().await;
                return;
            }
            Some(StopReason::Shutdown) => {
                job.status = JobStatus::Queued;
                job.stage = JobStage::Waiting;
                job.attempts = job.attempts.saturating_sub(1);
                job.progress = 0.0;
                job.overall_progress = 0.0;
            }
            Some(StopReason::User) => {
                job.status = JobStatus::Cancelled;
                job.stage = JobStage::Waiting;
                job.finished_at = Some(now);
            }
            None => self.apply_failure(&mut job, &error, heal_outcome),
        }
        let status = job.status;
        let scheduled = status == JobStatus::Queued && job.stage == JobStage::WaitingRetry;
        let attempts = job.attempts;
        if scheduled {
            self.state
                .lock()
                .expect("estado da fila")
                .waiting
                .insert(job.id.clone());
        }
        self.persist(&job).await;
        self.emit_job(&job);
        if scheduled {
            tokio::spawn(wait_retry(Arc::clone(self), job.id.clone(), attempts));
        }
        self.after_finish().await;
    }

    /// Decide o destino de um job que falhou (retry, autocura ou falha definitiva).
    fn apply_failure(&self, job: &mut Job, error: &DownloadError, heal: Option<HealOutcome>) {
        let max_attempts = self.settings.get().max_attempts;
        let now = job.updated_at;
        let fail = |job: &mut Job, message: &str| {
            job.status = JobStatus::Failed;
            job.stage = JobStage::Waiting;
            job.error_kind = Some(error.kind.as_str().to_string());
            job.error_message = Some(message.to_string());
            job.finished_at = Some(now);
        };
        match (error.kind, heal) {
            (_, Some(HealOutcome::Requeue)) => {
                // Reenfileira sem contar a tentativa que o conserto invalidou.
                job.status = JobStatus::Queued;
                job.stage = JobStage::Waiting;
                job.attempts = job.attempts.saturating_sub(1);
                job.progress = 0.0;
                job.overall_progress = 0.0;
            }
            (_, Some(HealOutcome::FailPersistent)) => {
                fail(job, "errors.extractorPersistent");
            }
            (ErrorKind::Network | ErrorKind::Unknown | ErrorKind::Ffmpeg, _) => {
                // `unknown` e `ffmpeg` têm uma única repetição (§9).
                let limit = if error.kind == ErrorKind::Network {
                    max_attempts
                } else {
                    max_attempts.min(2)
                };
                if job.attempts < limit {
                    job.status = JobStatus::Queued;
                    job.stage = JobStage::WaitingRetry;
                    job.error_kind = Some(error.kind.as_str().to_string());
                    job.error_message = Some(error.message.clone());
                    job.progress = 0.0;
                    job.overall_progress = 0.0;
                } else {
                    fail(job, &error.message);
                }
            }
            _ => fail(job, &error.message),
        }
    }
}

/// Acrescenta um estágio que só se revela durante o job, antes de `Moving` (que fica por último).
fn insert_before_moving(present: &mut Vec<JobStage>, stage: JobStage) {
    if !present.contains(&stage) {
        let at = present.len().saturating_sub(1);
        present.insert(at, stage);
    }
}

/// Estágios que o job deve atravessar, para o progresso geral. Só o perfil `original` pode (ou
/// não) converter; ele só vira `Converting` se o pipeline emitir conversão.
fn planned_stages(profile_id: &str) -> Vec<JobStage> {
    if profile_id == "original" {
        vec![JobStage::Downloading, JobStage::Moving]
    } else {
        vec![
            JobStage::Downloading,
            JobStage::Converting,
            JobStage::Moving,
        ]
    }
}

async fn run_loop(inner: Arc<Inner>) {
    loop {
        tokio::select! {
            () = inner.stop.cancelled() => return,
            () = inner.wake.notified() => {}
        }
        inner.start_ready().await;
    }
}

async fn wait_retry(inner: Arc<Inner>, id: String, attempts: u32) {
    tokio::select! {
        () = tokio::time::sleep(retry_delay(attempts)) => {}
        () = inner.stop.cancelled() => return,
    }
    let still_waiting = inner
        .state
        .lock()
        .expect("estado da fila")
        .waiting
        .remove(&id);
    if !still_waiting {
        return; // cancelado ou removido durante a espera
    }
    let owned = id.clone();
    let updated = inner
        .db
        .call(move |conn| {
            let Some(mut job) = repo::get(conn, &owned)? else {
                return Ok(None);
            };
            if job.status != JobStatus::Queued {
                return Ok(None);
            }
            job.stage = JobStage::Waiting;
            job.updated_at = repo::now();
            repo::save(conn, &job)?;
            Ok(Some(job))
        })
        .await;
    if let Ok(Some(job)) = updated {
        inner.emit_job(&job);
    }
    inner.wake.notify_one();
}

fn sponsorblock_categories(job: &Job, inner: &Inner) -> Option<Vec<String>> {
    let settings = inner.settings.get();
    let enabled = job
        .options
        .sponsorblock
        .unwrap_or(settings.sponsorblock_remove);
    enabled.then(|| settings.sponsorblock_categories.clone())
}

async fn run_job(inner: Arc<Inner>, job: Job, cancel: CancellationToken) {
    let id = job.id.clone();
    let Some(chosen) = profile(&job.profile_id) else {
        let error = DownloadError::new(
            ErrorKind::Unknown,
            format!("perfil desconhecido: {}", job.profile_id),
        );
        inner.finish_err(&id, error).await;
        return;
    };
    let settings = inner.settings.get();
    let out_dir = match output_dir(&inner.db, &job, &settings).await {
        Ok(path) => path,
        Err(error) => {
            inner
                .finish_err(&id, DownloadError::new(ErrorKind::Disk, error.to_string()))
                .await;
            return;
        }
    };
    let pipeline_job = PipelineJob {
        job_id: id.clone(),
        url: job.source_url.clone(),
        profile: chosen,
        out_dir,
        sponsorblock: sponsorblock_categories(&job, &inner),
        metadata_override: job.metadata_override.clone(),
        fetch_metadata: job.options.fetch_metadata,
        settings: Some(settings),
        options: job.options.clone(),
        playlist_ctx: job.playlist_ctx.clone(),
    };
    let on_event = |event| inner.on_event(&id, event);
    let result = inner.runner.run(&pipeline_job, &cancel, &on_event).await;
    match result {
        Ok(output) => inner.finish_ok(&id, output).await,
        Err(error) => inner.finish_err(&id, error).await,
    }
}

pub(crate) async fn output_dir(
    db: &Db,
    job: &Job,
    settings: &crate::Settings,
) -> CoreResult<PathBuf> {
    if let Some(dir) = job
        .options
        .output_dir
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        return Ok(PathBuf::from(dir));
    }
    if let Some(id) = &job.sync_id {
        let id = id.clone();
        let dir = db
            .call(move |conn| {
                let mut stmt = conn.prepare_cached("SELECT output_dir FROM syncs WHERE id=?1")?;
                let mut rows = stmt.query([id])?;
                Ok(rows
                    .next()?
                    .map(|row| row.get::<_, Option<String>>(0))
                    .transpose()?
                    .flatten())
            })
            .await?;
        if let Some(dir) = dir.filter(|s| !s.trim().is_empty()) {
            return Ok(PathBuf::from(dir));
        }
    }
    Ok(resolve_output_dir(settings))
}

//! F04 — T1–T14: a fila inteira com `FakeBackend`, banco em memória e o relógio do Tokio pausado.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::time::Instant;

use super::fake::{FakeBackend, Script};
use super::*;
use crate::error::{CoreError, CoreResult};
use crate::events::MemorySink;
use crate::pipeline::DownloadPipeline;
use crate::settings::{PotProvider, SettingsService, YtdlpChannel};
use crate::ytdlp::errors::ErrorKind;
use crate::Db;

// ------------------------------------------------------------------ mock da autocura

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fix {
    Update,
    Nightly,
    Pot,
    Never,
}

struct MockHeal {
    updates: AtomicU32,
    nightly: AtomicU32,
    pot_calls: AtomicU32,
    /// `update_ytdlp` devolve "instalou versão nova".
    update_installs: AtomicBool,
    channel: Mutex<YtdlpChannel>,
    provider: Mutex<PotProvider>,
    pot_on: AtomicBool,
    fix_on: Fix,
    ok: Arc<AtomicBool>,
}

impl MockHeal {
    fn new(fix_on: Fix) -> Arc<Self> {
        Arc::new(Self {
            updates: AtomicU32::new(0),
            nightly: AtomicU32::new(0),
            pot_calls: AtomicU32::new(0),
            update_installs: AtomicBool::new(true),
            channel: Mutex::new(YtdlpChannel::Stable),
            provider: Mutex::new(PotProvider::Auto),
            pot_on: AtomicBool::new(false),
            fix_on,
            ok: Arc::new(AtomicBool::new(false)),
        })
    }

    fn updates(&self) -> u32 {
        self.updates.load(Ordering::SeqCst)
    }

    fn nightly(&self) -> u32 {
        self.nightly.load(Ordering::SeqCst)
    }

    fn pot_calls(&self) -> u32 {
        self.pot_calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl HealTools for MockHeal {
    async fn update_ytdlp(&self) -> CoreResult<bool> {
        self.updates.fetch_add(1, Ordering::SeqCst);
        if self.fix_on == Fix::Update {
            self.ok.store(true, Ordering::SeqCst);
        }
        Ok(self.update_installs.load(Ordering::SeqCst))
    }

    async fn switch_to_nightly(&self) -> CoreResult<()> {
        self.nightly.fetch_add(1, Ordering::SeqCst);
        *self.channel.lock().unwrap() = YtdlpChannel::Nightly;
        if self.fix_on == Fix::Nightly {
            self.ok.store(true, Ordering::SeqCst);
        }
        Ok(())
    }

    fn channel(&self) -> YtdlpChannel {
        *self.channel.lock().unwrap()
    }

    fn pot_provider(&self) -> PotProvider {
        *self.provider.lock().unwrap()
    }

    async fn pot_active(&self) -> bool {
        self.pot_on.load(Ordering::SeqCst)
    }

    async fn enable_pot(&self) -> CoreResult<()> {
        self.pot_calls.fetch_add(1, Ordering::SeqCst);
        self.pot_on.store(true, Ordering::SeqCst);
        if self.fix_on == Fix::Pot {
            self.ok.store(true, Ordering::SeqCst);
        }
        Ok(())
    }
}

// ------------------------------------------------------------------ ambiente de teste

struct Env {
    dir: tempfile::TempDir,
    db: Db,
    sink: Arc<MemorySink>,
    settings: Arc<SettingsService>,
    backend: Arc<FakeBackend>,
    heal_tools: Arc<MockHeal>,
    heal: Arc<HealCoordinator>,
    clock: Arc<AtomicU64>,
    queue: QueueService,
}

impl Env {
    async fn new() -> Self {
        Self::with_heal(Fix::Never).await
    }

    async fn with_heal(fix_on: Fix) -> Self {
        let env = Self::without_service(fix_on).await;
        let queue = env.start().await;
        Self { queue, ..env }
    }

    /// Tudo pronto, menos o serviço (o `queue` do resultado é um serviço parado e pausado).
    async fn without_service(fix_on: Fix) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let sink = Arc::new(MemorySink::new());
        let db = Db::open_in_memory().unwrap();
        let settings = Arc::new(
            SettingsService::new(db.clone(), sink.clone())
                .await
                .unwrap(),
        );
        let out = dir.path().join("musicas");
        settings
            .update(serde_json::from_value(json!({ "outputDir": out })).unwrap())
            .await
            .unwrap();
        let backend = FakeBackend::new();
        let heal_tools = MockHeal::new(fix_on);
        // O relógio da autocura só anda quando o teste manda.
        let clock = Arc::new(AtomicU64::new(1_000_000));
        let heal = Arc::new(HealCoordinator::with_clock(
            heal_tools.clone(),
            db.clone(),
            sink.clone(),
            {
                let clock = Arc::clone(&clock);
                Arc::new(move || clock.load(Ordering::SeqCst))
            },
        ));
        let placeholder = QueueService::start(deps(
            &db,
            &settings,
            &sink,
            &backend,
            &heal,
            &dir.path().join("placeholder"),
        ))
        .await
        .unwrap();
        placeholder.pause().await;
        Self {
            dir,
            db,
            sink,
            settings,
            backend,
            heal_tools,
            heal,
            clock,
            queue: placeholder,
        }
    }

    async fn start(&self) -> QueueService {
        QueueService::start(deps(
            &self.db,
            &self.settings,
            &self.sink,
            &self.backend,
            &self.heal,
            self.dir.path(),
        ))
        .await
        .unwrap()
    }

    fn out_dir(&self) -> PathBuf {
        self.dir.path().join("musicas")
    }

    async fn set(&self, patch: Value) {
        self.settings
            .update(serde_json::from_value(patch).unwrap())
            .await
            .unwrap();
        self.queue.wake();
    }

    async fn job(&self, id: &str) -> Job {
        self.queue.get(id).await.unwrap().expect("job existe")
    }

    /// Espera (em tempo virtual) o job chegar ao status.
    async fn wait_status(&self, id: &str, status: JobStatus) -> Job {
        let deadline = Instant::now() + Duration::from_secs(900);
        loop {
            let job = self.job(id).await;
            if job.status == status {
                return job;
            }
            assert!(
                Instant::now() < deadline,
                "job {id} não chegou a {status:?}: {:?}/{:?} ({:?})",
                job.status,
                job.stage,
                job.error_message
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn wait_until(&self, what: &str, mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(900);
        while !condition() {
            assert!(Instant::now() < deadline, "tempo esgotado: {what}");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn wait_idle(&self) {
        let deadline = Instant::now() + Duration::from_secs(900);
        loop {
            let jobs = self.queue.list().await.unwrap();
            if jobs
                .iter()
                .all(|j| !matches!(j.status, JobStatus::Queued | JobStatus::Running))
            {
                return;
            }
            assert!(Instant::now() < deadline, "a fila não esvaziou: {jobs:#?}");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

fn deps(
    db: &Db,
    settings: &Arc<SettingsService>,
    sink: &Arc<MemorySink>,
    backend: &Arc<FakeBackend>,
    heal: &Arc<HealCoordinator>,
    data_dir: &std::path::Path,
) -> QueueDeps {
    let pipeline = DownloadPipeline::new(
        backend.clone(),
        data_dir.to_path_buf(),
        PathBuf::from("/nada/ffmpeg"),
        None,
    );
    QueueDeps {
        db: db.clone(),
        settings: Arc::clone(settings),
        sink: sink.clone(),
        runner: Arc::new(pipeline),
        heal: Arc::clone(heal),
        data_dir: data_dir.to_path_buf(),
        start_paused: false,
    }
}

fn url(n: u32) -> String {
    format!("https://www.youtube.com/watch?v=vid{n}")
}

fn request(n: u32) -> EnqueueRequest {
    EnqueueRequest {
        url: url(n),
        source_id: Some(format!("vid{n}")),
        ..EnqueueRequest::default()
    }
}

// ------------------------------------------------------------------ T1–T3

#[tokio::test(start_paused = true)]
async fn t1_nunca_passa_do_limite_de_paralelismo() {
    let env = Env::new().await;
    env.set(json!({ "parallelism": 2 })).await;
    for n in 1..=6 {
        env.backend.script(&url(n), Script::ok(1000));
        env.queue.enqueue(request(n)).await.unwrap();
    }
    env.wait_idle().await;

    assert_eq!(env.backend.max_running(), 2);
    let jobs = env.queue.list().await.unwrap();
    assert_eq!(jobs.len(), 6);
    assert!(jobs.iter().all(|j| j.status == JobStatus::Done));
    // Nenhum evento mostrou mais de 2 jobs rodando ao mesmo tempo.
    let mut running = std::collections::HashSet::new();
    let mut peak = 0;
    for event in env.sink.named("job://updated") {
        let id = event["id"].as_str().unwrap().to_string();
        if event["status"] == "running" {
            running.insert(id);
        } else {
            running.remove(&id);
        }
        peak = peak.max(running.len());
    }
    assert!(peak <= 2, "pico observado nos eventos: {peak}");
}

#[tokio::test(start_paused = true)]
async fn t2_parallelism_muda_em_tempo_real() {
    let env = Env::new().await;
    env.set(json!({ "parallelism": 1 })).await;
    for n in 1..=6 {
        env.backend.script(&url(n), Script::ok(10_000));
        env.queue.enqueue(request(n)).await.unwrap();
    }
    env.wait_until("1 rodando", || env.backend.running() == 1)
        .await;
    assert_eq!(env.backend.running(), 1);

    env.set(json!({ "parallelism": 3 })).await;
    env.wait_until("3 rodando", || env.backend.running() == 3)
        .await;

    // Reduzir para 1 não interrompe os 3 em execução…
    env.set(json!({ "parallelism": 1 })).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(env.backend.running(), 3);
    // …e os que sobraram só entram quando houver vaga dentro do novo limite.
    env.wait_idle().await;
    assert_eq!(env.backend.max_running(), 3);
    let jobs = env.queue.list().await.unwrap();
    assert!(jobs.iter().all(|j| j.status == JobStatus::Done));
}

#[tokio::test(start_paused = true)]
async fn t3_pausar_e_retomar() {
    let env = Env::new().await;
    env.set(json!({ "parallelism": 1 })).await;
    let mut ids = Vec::new();
    for n in 1..=3 {
        env.backend.script(&url(n), Script::ok(1000));
        ids.push(env.queue.enqueue(request(n)).await.unwrap().id);
    }
    env.wait_until("1 rodando", || env.backend.running() == 1)
        .await;
    env.queue.pause().await;
    assert!(env.queue.state().await.unwrap().paused);

    // O que já rodava termina; nada novo começa.
    env.wait_status(&ids[0], JobStatus::Done).await;
    tokio::time::sleep(Duration::from_secs(30)).await;
    assert_eq!(env.job(&ids[1]).await.status, JobStatus::Queued);
    assert_eq!(env.job(&ids[2]).await.status, JobStatus::Queued);
    assert_eq!(env.backend.call_count(&url(2)), 0);

    env.queue.resume().await;
    env.wait_idle().await;
    for id in &ids {
        assert_eq!(env.job(id).await.status, JobStatus::Done);
    }
}

// ------------------------------------------------------------------ T4

#[tokio::test(start_paused = true)]
async fn t4_cancelar_em_fila_e_em_execucao() {
    let env = Env::new().await;
    env.set(json!({ "parallelism": 1 })).await;
    env.backend.script(&url(1), Script::Hang);
    let running = env.queue.enqueue(request(1)).await.unwrap();
    let queued = env.queue.enqueue(request(2)).await.unwrap();
    env.wait_until("job 1 rodando", || env.backend.running() == 1)
        .await;
    let workspace = env.dir.path().join("tmp").join(&running.id);
    assert!(workspace.exists());

    env.queue.cancel(&queued.id).await.unwrap();
    assert_eq!(env.job(&queued.id).await.status, JobStatus::Cancelled);

    env.queue.cancel(&running.id).await.unwrap();
    env.wait_status(&running.id, JobStatus::Cancelled).await;
    assert!(!workspace.exists(), "o workspace do job deve ser apagado");
    assert_eq!(env.backend.running(), 0);
    assert_eq!(env.queue.state().await.unwrap().running, 0);

    // O contador foi liberado: o próximo job entra.
    let next = env.queue.enqueue(request(3)).await.unwrap();
    env.wait_status(&next.id, JobStatus::Done).await;
}

// ------------------------------------------------------------------ T5–T6

#[tokio::test(start_paused = true)]
async fn t5_retry_com_backoff_de_5s_e_30s() {
    let env = Env::new().await;
    env.set(json!({ "maxAttempts": 3 })).await;
    env.backend
        .script(&url(1), Script::fail_then_ok(ErrorKind::Network, 2));
    env.backend
        .script(&url(2), Script::fail(ErrorKind::Network));
    let recovers = env.queue.enqueue(request(1)).await.unwrap();
    let gives_up = env.queue.enqueue(request(2)).await.unwrap();

    let done = env.wait_status(&recovers.id, JobStatus::Done).await;
    assert_eq!(done.attempts, 3);
    let calls = env.backend.calls(&url(1));
    assert_eq!(calls.len(), 3);
    let first = calls[1] - calls[0];
    let second = calls[2] - calls[1];
    assert!(
        first >= Duration::from_secs(5) && first < Duration::from_millis(5500),
        "primeira espera: {first:?}"
    );
    assert!(
        second >= Duration::from_secs(30) && second < Duration::from_millis(30_500),
        "segunda espera: {second:?}"
    );
    // Durante a espera o job ficou `queued` com `waiting_retry`.
    let waited = env.sink.named("job://updated").iter().any(|e| {
        e["id"] == recovers.id.as_str() && e["status"] == "queued" && e["stage"] == "waiting_retry"
    });
    assert!(waited);

    let failed = env.wait_status(&gives_up.id, JobStatus::Failed).await;
    assert_eq!(failed.attempts, 3);
    assert_eq!(failed.error_kind.as_deref(), Some("network"));
    assert_eq!(env.backend.call_count(&url(2)), 3);
}

#[tokio::test(start_paused = true)]
async fn t5b_unknown_tem_uma_unica_repeticao() {
    let env = Env::new().await;
    env.set(json!({ "maxAttempts": 5 })).await;
    env.backend
        .script(&url(1), Script::fail(ErrorKind::Unknown));
    let job = env.queue.enqueue(request(1)).await.unwrap();
    let failed = env.wait_status(&job.id, JobStatus::Failed).await;
    assert_eq!(failed.attempts, 2);
    assert_eq!(env.backend.call_count(&url(1)), 2);
}

#[tokio::test(start_paused = true)]
async fn t6_erros_permanentes_nao_repetem() {
    let env = Env::new().await;
    env.set(json!({ "potProvider": "off", "maxAttempts": 5 }))
        .await;
    let kinds = [
        ErrorKind::Unavailable,
        ErrorKind::AgeRestricted,
        ErrorKind::BotCheck,
        ErrorKind::GeoBlocked,
        ErrorKind::Disk,
    ];
    for (offset, kind) in kinds.into_iter().enumerate() {
        let n = offset as u32 + 1;
        *env.heal_tools.provider.lock().unwrap() = PotProvider::Off;
        env.backend.script(&url(n), Script::fail(kind));
        let job = env.queue.enqueue(request(n)).await.unwrap();
        let failed = env.wait_status(&job.id, JobStatus::Failed).await;
        assert_eq!(failed.attempts, 1, "{kind:?}");
        assert_eq!(failed.error_kind.as_deref(), Some(kind.as_str()));
        assert_eq!(env.backend.call_count(&url(n)), 1, "{kind:?}");
    }
}

// ------------------------------------------------------------------ T7–T8b (autocura)

fn extractor_until(ok: &Arc<AtomicBool>) -> Script {
    Script::FailUntil {
        kind: ErrorKind::Extractor,
        stderr: "ERROR: Unable to extract nsig",
        ok: Arc::clone(ok),
    }
}

#[tokio::test(start_paused = true)]
async fn t7_tres_falhas_simultaneas_atualizam_uma_vez() {
    let env = Env::with_heal(Fix::Update).await;
    env.set(json!({ "parallelism": 3 })).await;
    let mut ids = Vec::new();
    for n in 1..=3 {
        env.backend
            .script(&url(n), extractor_until(&env.heal_tools.ok));
        ids.push(env.queue.enqueue(request(n)).await.unwrap().id);
    }
    for id in &ids {
        let job = env.wait_status(id, JobStatus::Done).await;
        assert_eq!(job.attempts, 1, "o conserto não consome tentativa");
    }
    assert_eq!(env.heal_tools.updates(), 1);
    assert_eq!(env.heal_tools.nightly(), 0);
    let states: Vec<_> = env
        .sink
        .named("heal://state")
        .iter()
        .map(|e| e["stage"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(states.first().map(String::as_str), Some("checking"));
    assert_eq!(states.last().map(String::as_str), Some("done"));
}

#[tokio::test(start_paused = true)]
async fn t8_escala_para_o_nightly() {
    let env = Env::with_heal(Fix::Nightly).await;
    env.backend
        .script(&url(1), extractor_until(&env.heal_tools.ok));
    let job = env.queue.enqueue(request(1)).await.unwrap();
    env.wait_status(&job.id, JobStatus::Done).await;

    assert_eq!(env.heal_tools.updates(), 1);
    assert_eq!(env.heal_tools.nightly(), 1);
    assert!(env
        .db
        .kv_get("heal_switched_to_nightly")
        .await
        .unwrap()
        .is_some());
    let notices = env.sink.named("notice");
    assert!(notices
        .iter()
        .any(|n| n["i18nKey"] == "notices.ytdlpNightly"));
}

#[tokio::test(start_paused = true)]
async fn t8_sem_versao_nova_vai_direto_para_o_nightly() {
    let env = Env::with_heal(Fix::Nightly).await;
    env.heal_tools
        .update_installs
        .store(false, Ordering::SeqCst);
    env.backend
        .script(&url(1), extractor_until(&env.heal_tools.ok));
    let job = env.queue.enqueue(request(1)).await.unwrap();
    env.wait_status(&job.id, JobStatus::Done).await;
    assert_eq!(env.heal_tools.updates(), 1);
    assert_eq!(env.heal_tools.nightly(), 1);
}

#[tokio::test(start_paused = true)]
async fn t8_nightly_tambem_falha_e_nao_repete_dentro_de_1h() {
    let env = Env::with_heal(Fix::Never).await;
    env.backend
        .script(&url(1), extractor_until(&env.heal_tools.ok));
    let first = env.queue.enqueue(request(1)).await.unwrap();
    let failed = env.wait_status(&first.id, JobStatus::Failed).await;
    assert_eq!(failed.error_kind.as_deref(), Some("extractor"));
    assert_eq!(
        failed.error_message.as_deref(),
        Some("errors.extractorPersistent")
    );
    assert_eq!((env.heal_tools.updates(), env.heal_tools.nightly()), (1, 1));
    let states: Vec<_> = env
        .sink
        .named("heal://state")
        .iter()
        .map(|e| e["stage"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(states.last().map(String::as_str), Some("failed"));

    // Segunda falha dentro de 1 h: nenhuma autocura nova.
    env.clock.fetch_add(1800, Ordering::SeqCst);
    env.backend
        .script(&url(2), extractor_until(&env.heal_tools.ok));
    let second = env.queue.enqueue(request(2)).await.unwrap();
    let failed = env.wait_status(&second.id, JobStatus::Failed).await;
    assert_eq!(
        failed.error_message.as_deref(),
        Some("errors.extractorPersistent")
    );
    assert_eq!((env.heal_tools.updates(), env.heal_tools.nightly()), (1, 1));

    // Passada 1 h, a autocura volta a poder rodar.
    env.clock.fetch_add(3600, Ordering::SeqCst);
    env.backend
        .script(&url(3), extractor_until(&env.heal_tools.ok));
    let third = env.queue.enqueue(request(3)).await.unwrap();
    env.wait_status(&third.id, JobStatus::Failed).await;
    assert!(env.heal_tools.updates() >= 2);
}

#[tokio::test(start_paused = true)]
async fn t8b_po_token_bot_check_liga_o_provedor() {
    let env = Env::with_heal(Fix::Pot).await;
    env.backend.script(
        &url(1),
        Script::FailUntil {
            kind: ErrorKind::BotCheck,
            stderr: "Sign in to confirm you're not a bot",
            ok: Arc::clone(&env.heal_tools.ok),
        },
    );
    let job = env.queue.enqueue(request(1)).await.unwrap();
    let done = env.wait_status(&job.id, JobStatus::Done).await;
    assert_eq!(done.attempts, 1);
    assert_eq!(env.heal_tools.pot_calls(), 1);
    assert_eq!(env.heal_tools.updates(), 0);
}

#[tokio::test(start_paused = true)]
async fn t8b_provedor_ja_ligado_ou_desligado_falha_direto() {
    // Já ligado: sem novo passo 0.
    let env = Env::with_heal(Fix::Never).await;
    env.heal_tools.pot_on.store(true, Ordering::SeqCst);
    env.backend
        .script(&url(1), Script::fail(ErrorKind::BotCheck));
    let job = env.queue.enqueue(request(1)).await.unwrap();
    let failed = env.wait_status(&job.id, JobStatus::Failed).await;
    assert_eq!(failed.error_kind.as_deref(), Some("bot_check"));
    assert_eq!(env.heal_tools.pot_calls(), 0);
    assert_eq!(env.heal_tools.updates(), 0);

    // `potProvider = off`: falha direto.
    let env = Env::with_heal(Fix::Never).await;
    *env.heal_tools.provider.lock().unwrap() = PotProvider::Off;
    env.backend
        .script(&url(1), Script::fail(ErrorKind::BotCheck));
    let job = env.queue.enqueue(request(1)).await.unwrap();
    let failed = env.wait_status(&job.id, JobStatus::Failed).await;
    assert_eq!(failed.error_kind.as_deref(), Some("bot_check"));
    assert_eq!(env.heal_tools.pot_calls(), 0);
}

#[tokio::test(start_paused = true)]
async fn t8b_extractor_403_tenta_o_po_token_antes_do_update() {
    let env = Env::with_heal(Fix::Pot).await;
    env.backend.script(
        &url(1),
        Script::FailUntil {
            kind: ErrorKind::Extractor,
            stderr: "ERROR: unable to download video data: HTTP Error 403: Forbidden",
            ok: Arc::clone(&env.heal_tools.ok),
        },
    );
    let job = env.queue.enqueue(request(1)).await.unwrap();
    env.wait_status(&job.id, JobStatus::Done).await;
    assert_eq!(env.heal_tools.pot_calls(), 1);
    assert_eq!(env.heal_tools.updates(), 0);
}

// ------------------------------------------------------------------ T9–T10

fn raw_job(id: &str, status: JobStatus, stage: JobStage, position: i64, n: u32) -> Job {
    Job {
        id: id.to_string(),
        kind: "single".to_string(),
        provider: "youtube".to_string(),
        source_url: url(n),
        source_id: Some(format!("vid{n}")),
        title: None,
        artist: None,
        thumbnail: None,
        duration_s: None,
        profile_id: "original".to_string(),
        options: JobOptions::default(),
        metadata_override: None,
        confidence: None,
        metadata_result: None,
        warnings: Vec::new(),
        playlist_ctx: None,
        sync_id: None,
        status,
        stage,
        progress: 0.4,
        overall_progress: 0.3,
        speed_bps: Some(1.0),
        eta_s: Some(3),
        error_kind: None,
        error_message: None,
        attempts: 1,
        output_path: None,
        library_id: None,
        position,
        created_at: 10,
        updated_at: 10,
        finished_at: None,
    }
}

#[tokio::test(start_paused = true)]
async fn t9_restaura_jobs_em_execucao_ao_recriar_o_servico() {
    let env = Env::without_service(Fix::Never).await;
    env.db
        .call(|conn| {
            repo::insert(
                conn,
                &raw_job("a", JobStatus::Running, JobStage::Downloading, 1, 1),
            )?;
            repo::insert(
                conn,
                &raw_job("b", JobStatus::Queued, JobStage::WaitingRetry, 2, 2),
            )?;
            repo::insert(conn, &raw_job("c", JobStatus::Done, JobStage::Done, 3, 3))?;
            Ok(())
        })
        .await
        .unwrap();
    // Sobra de uma execução anterior.
    std::fs::create_dir_all(env.dir.path().join("tmp").join("a")).unwrap();

    let queue = env.start().await;
    let restored = queue.get("a").await.unwrap().unwrap();
    assert_ne!(restored.status, JobStatus::Failed);
    for id in ["a", "b"] {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let job = queue.get(id).await.unwrap().unwrap();
            if job.status == JobStatus::Done {
                assert_eq!(
                    job.attempts, 2,
                    "a tentativa interrompida ficou contada (+1 da nova)"
                );
                break;
            }
            assert!(Instant::now() < deadline, "{id} não terminou: {job:?}");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    assert!(!env.dir.path().join("tmp").join("a").exists());
    assert_eq!(
        queue.get("c").await.unwrap().unwrap().status,
        JobStatus::Done
    );
}

#[tokio::test(start_paused = true)]
async fn t10_reordenar_e_baixar_agora() {
    let env = Env::new().await;
    env.set(json!({ "parallelism": 1 })).await;
    env.queue.pause().await;
    let mut ids = Vec::new();
    for n in 1..=3 {
        ids.push(env.queue.enqueue(request(n)).await.unwrap().id);
    }
    // 3 passa na frente do 1.
    env.queue
        .move_job(&ids[2], MoveTarget::Before(ids[0].clone()))
        .await
        .unwrap();
    // "Baixar agora" fura a fila inteira.
    let urgent = env
        .queue
        .enqueue(EnqueueRequest {
            priority: true,
            ..request(4)
        })
        .await
        .unwrap();
    assert!(urgent.position < env.job(&ids[2]).await.position);

    env.queue.resume().await;
    env.wait_idle().await;
    assert_eq!(env.backend.order(), vec![url(4), url(3), url(1), url(2)]);
}

#[tokio::test(start_paused = true)]
async fn t10_mover_depois_e_para_o_fim() {
    let env = Env::new().await;
    env.queue.pause().await;
    let mut ids = Vec::new();
    for n in 1..=4 {
        ids.push(env.queue.enqueue(request(n)).await.unwrap().id);
    }
    env.queue
        .move_job(&ids[0], MoveTarget::After(ids[2].clone()))
        .await
        .unwrap();
    env.queue.move_job(&ids[1], MoveTarget::Back).await.unwrap();
    let order: Vec<_> = env
        .queue
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|j| j.id)
        .collect();
    assert_eq!(
        order,
        vec![
            ids[2].clone(),
            ids[0].clone(),
            ids[3].clone(),
            ids[1].clone()
        ]
    );
    let err = env
        .queue
        .move_job("nao-existe", MoveTarget::Front)
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "invalid");
}

// ------------------------------------------------------------------ T11–T12

#[tokio::test(start_paused = true)]
async fn t11_respeita_o_limite_da_fila() {
    let env = Env::new().await;
    env.set(json!({ "queueLimit": 10 })).await;
    env.queue.pause().await;
    let mut first = None;
    for n in 1..=10 {
        let job = env.queue.enqueue(request(n)).await.unwrap();
        first.get_or_insert(job.id);
    }
    let err = env.queue.enqueue(request(11)).await.unwrap_err();
    assert_eq!(err.kind(), "queue_full");
    assert_eq!(env.queue.list().await.unwrap().len(), 10);

    env.queue.cancel(&first.unwrap()).await.unwrap();
    env.queue.enqueue(request(11)).await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn t12_duplicatas() {
    let env = Env::new().await;
    env.queue.pause().await;
    env.queue.enqueue(request(1)).await.unwrap();

    let err = env.queue.enqueue(request(1)).await.unwrap_err();
    assert_eq!(err.kind(), "duplicate");
    assert!(err.to_string().contains("vid1"));

    env.queue
        .enqueue(EnqueueRequest {
            allow_duplicate: true,
            ..request(1)
        })
        .await
        .unwrap();
    env.queue
        .enqueue(EnqueueRequest {
            profile_id: Some("mp3_v0".to_string()),
            ..request(1)
        })
        .await
        .unwrap();

    // Cancelados e falhos não contam como duplicata.
    let other = env.queue.enqueue(request(2)).await.unwrap();
    env.queue.cancel(&other.id).await.unwrap();
    env.queue.enqueue(request(2)).await.unwrap();

    // A biblioteca também.
    env.db
        .call(|conn| {
            crate::library::insert_from_job(
                conn,
                &raw_job("library-job", JobStatus::Done, JobStage::Done, 1, 9),
                &crate::library::DownloadedFile {
                    file_path: "/m/a.opus".into(),
                    tags: crate::tagging::TrackTags {
                        title: "A".into(),
                        ..Default::default()
                    },
                    probe: None,
                    source_abr_kbps: None,
                    content_type: crate::metadata::ContentType::Music,
                    has_synced_lyrics: false,
                    cover_source: None,
                    replaygain_db: None,
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let err = env.queue.enqueue(request(9)).await.unwrap_err();
    assert_eq!(err.kind(), "duplicate");
    let hits = env
        .queue
        .check_duplicates(vec!["vid1".into(), "vid9".into(), "vid77".into()], None)
        .await
        .unwrap();
    let found: Vec<_> = hits
        .iter()
        .map(|h| (h.source_id.as_str(), h.found_in.as_str()))
        .collect();
    assert_eq!(found, vec![("vid1", "queue"), ("vid9", "library")]);
}

// ------------------------------------------------------------------ T13–T14

#[tokio::test(start_paused = true)]
async fn t13_throttle_de_eventos_de_progresso() {
    let env = Env::new().await;
    // 100 progressos por segundo durante 10 s.
    env.backend.script(
        &url(1),
        Script::Ok {
            ms: 10_000,
            steps: 1000,
        },
    );
    let job = env.queue.enqueue(request(1)).await.unwrap();
    env.wait_status(&job.id, JobStatus::Done).await;

    let events: Vec<Value> = env
        .sink
        .named("job://updated")
        .into_iter()
        .filter(|e| e["id"] == job.id.as_str())
        .collect();
    let progress_events = events.iter().filter(|e| e["status"] == "running").count();
    assert!(
        progress_events <= 5 * 10 + 2,
        "{progress_events} eventos de progresso em 10 s"
    );
    assert!(
        progress_events >= 10,
        "ainda precisa haver progresso visível"
    );
    // Mudanças de status sempre saem.
    let statuses: Vec<_> = events
        .iter()
        .map(|e| e["status"].as_str().unwrap())
        .collect();
    assert_eq!(statuses.first(), Some(&"queued"));
    assert!(statuses.contains(&"running"));
    assert_eq!(statuses.last(), Some(&"done"));
}

#[tokio::test(start_paused = true)]
async fn t14_progresso_geral_monotonico_de_0_a_1() {
    let env = Env::new().await;
    env.backend.script(
        &url(1),
        Script::Ok {
            ms: 5_000,
            steps: 500,
        },
    );
    let job = env.queue.enqueue(request(1)).await.unwrap();
    env.wait_status(&job.id, JobStatus::Done).await;

    let values: Vec<f64> = env
        .sink
        .named("job://updated")
        .into_iter()
        .filter(|e| e["id"] == job.id.as_str())
        .map(|e| e["overallProgress"].as_f64().unwrap())
        .collect();
    assert!(values.len() > 5);
    assert_eq!(values.first(), Some(&0.0));
    assert_eq!(values.last(), Some(&1.0));
    assert!(values.windows(2).all(|w| w[1] >= w[0]), "{values:?}");
}

// ------------------------------------------------------------------ operações restantes

#[tokio::test(start_paused = true)]
async fn conclui_move_o_arquivo_e_retry_remove_e_limpeza() {
    let env = Env::new().await;
    env.set(json!({ "maxAttempts": 1 })).await;
    env.backend
        .script(&url(2), Script::fail(ErrorKind::Network));
    let ok = env.queue.enqueue(request(1)).await.unwrap();
    let bad = env.queue.enqueue(request(2)).await.unwrap();

    let done = env.wait_status(&ok.id, JobStatus::Done).await;
    assert_eq!(done.title.as_deref(), Some("Faixa vid1"));
    let output = PathBuf::from(done.output_path.clone().unwrap());
    assert_eq!(output, env.out_dir().join("Faixa vid1.opus"));
    assert!(output.is_file());
    assert!(!env.dir.path().join("tmp").join(&ok.id).exists());
    env.wait_status(&bad.id, JobStatus::Failed).await;

    // `retry` só vale para falhos/cancelados.
    assert_eq!(env.queue.retry(&ok.id).await.unwrap_err().kind(), "invalid");
    env.backend.script(&url(2), Script::ok(100));
    let retried = env.queue.retry(&bad.id).await.unwrap();
    assert_eq!((retried.status, retried.attempts), (JobStatus::Queued, 0));
    env.wait_status(&bad.id, JobStatus::Done).await;

    assert_eq!(env.queue.clear_finished().await.unwrap(), 2);
    assert!(env.queue.list().await.unwrap().is_empty());
    assert_eq!(env.sink.named("job://removed").len(), 2);

    // `remove` de um job em execução cancela e apaga.
    env.backend.script(&url(3), Script::Hang);
    let hanging = env.queue.enqueue(request(3)).await.unwrap();
    env.wait_until("rodando", || env.backend.running() == 1)
        .await;
    env.queue.remove(&hanging.id).await.unwrap();
    env.wait_until("removido", || env.backend.running() == 0)
        .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(env.queue.get(&hanging.id).await.unwrap().is_none());
}

#[tokio::test(start_paused = true)]
async fn cancel_all_cancela_tudo_e_shutdown_devolve_para_a_fila() {
    let env = Env::new().await;
    env.set(json!({ "parallelism": 2 })).await;
    for n in 1..=4 {
        env.backend.script(&url(n), Script::Hang);
        env.queue.enqueue(request(n)).await.unwrap();
    }
    env.wait_until("2 rodando", || env.backend.running() == 2)
        .await;
    env.queue.cancel_all().await.unwrap();
    env.wait_idle().await;
    let jobs = env.queue.list().await.unwrap();
    assert!(
        jobs.iter().all(|j| j.status == JobStatus::Cancelled),
        "{jobs:#?}"
    );

    // Fechar o app com um job rodando o devolve para `queued`, sem gastar tentativa.
    let env = Env::new().await;
    env.backend.script(&url(1), Script::Hang);
    let job = env.queue.enqueue(request(1)).await.unwrap();
    env.wait_until("rodando", || env.backend.running() == 1)
        .await;
    env.queue.shutdown().await;
    let stored = env
        .db
        .call(move |conn| repo::get(conn, &job.id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.status, JobStatus::Queued);
    assert_eq!(stored.attempts, 0);
}

#[tokio::test(start_paused = true)]
async fn enqueue_valida_a_entrada() {
    let env = Env::new().await;
    let empty = env
        .queue
        .enqueue(EnqueueRequest::default())
        .await
        .unwrap_err();
    assert!(matches!(empty, CoreError::Invalid { .. }));
    let bad_profile = env
        .queue
        .enqueue(EnqueueRequest {
            profile_id: Some("nao-existe".into()),
            ..request(1)
        })
        .await
        .unwrap_err();
    assert_eq!(bad_profile.kind(), "invalid");
}

mod metadata;
mod postprocess;
mod sync;

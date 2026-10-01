//! Provedor de PO token (bgutil): política de uso e ciclo de vida do servidor local
//! (estudo E4: <https://github.com/Brainicism/bgutil-ytdlp-pot-provider> usado só como programa).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use process_wrap::tokio::ChildWrapper;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use super::github::now_secs;
use super::process::{kill_tree, spawn_tool, tool_command};
use crate::db::Db;
use crate::error::{CoreError, CoreResult};
use crate::settings::PotProvider;

const KV_AUTO_UNTIL: &str = "pot.auto_until";
/// Com `potProvider = auto`, a autocura liga o provedor por 24 h.
pub const AUTO_TTL_SECS: u64 = 24 * 3600;

/// Argumentos extras do yt-dlp quando o provedor está ativo (E4). Não força `player_client`.
pub fn pot_args(plugin_dir: &Path, port: u16) -> Vec<String> {
    vec![
        "--plugin-dirs".to_string(),
        plugin_dir.display().to_string(),
        "--extractor-args".to_string(),
        format!("youtubepot-bgutilhttp:base_url=http://127.0.0.1:{port}"),
    ]
}

type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;

/// Política `potProvider`: `always`, `off` ou `auto` (ligado por 24 h após a autocura pedir).
#[derive(Clone)]
pub struct PotPolicy {
    db: Db,
    clock: Clock,
}

impl PotPolicy {
    pub fn new(db: Db) -> Self {
        Self::with_clock(db, Arc::new(now_secs))
    }

    /// Relógio injetável (segundos desde a época Unix), para testar a expiração.
    pub fn with_clock(db: Db, clock: Clock) -> Self {
        Self { db, clock }
    }

    /// Chamado pela autocura (F04) ao ver `bot_check` / 403 de formatos.
    pub async fn enable_auto(&self) -> CoreResult<()> {
        let until = (self.clock)() + AUTO_TTL_SECS;
        self.db.kv_set(KV_AUTO_UNTIL, &until.to_string()).await
    }

    pub async fn auto_until(&self) -> CoreResult<Option<u64>> {
        Ok(self
            .db
            .kv_get(KV_AUTO_UNTIL)
            .await?
            .and_then(|text| text.parse().ok()))
    }

    pub async fn is_active(&self, provider: PotProvider) -> CoreResult<bool> {
        Ok(match provider {
            PotProvider::Always => true,
            PotProvider::Off => false,
            PotProvider::Auto => self
                .auto_until()
                .await?
                .is_some_and(|until| (self.clock)() < until),
        })
    }
}

type ArgsFn = Arc<dyn Fn(u16) -> Vec<OsString> + Send + Sync>;

#[derive(Clone)]
pub struct PotServerConfig {
    pub program: PathBuf,
    /// Argumentos do processo para uma porta (ex.: `run … ../src/main.ts --port <porta>`).
    pub args: ArgsFn,
    pub cwd: Option<PathBuf>,
    /// Arquivo que recebe stdout/stderr do servidor (diagnóstico); sem ele a saída é descartada.
    pub log_file: Option<PathBuf>,
    pub ping_timeout: Duration,
    pub idle_timeout: Duration,
    pub idle_check: Duration,
}

impl PotServerConfig {
    pub fn new(program: PathBuf, args: ArgsFn, cwd: Option<PathBuf>) -> Self {
        Self {
            program,
            args,
            cwd,
            log_file: None,
            ping_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            idle_check: Duration::from_secs(15),
        }
    }
}

struct State {
    child: Option<Box<dyn ChildWrapper>>,
    port: u16,
    last_used: Instant,
    deaths: u32,
    given_up: bool,
    watcher: Option<JoinHandle<()>>,
}

/// Servidor HTTP do bgutil sob demanda, compartilhado entre os jobs.
pub struct PotServer {
    cfg: PotServerConfig,
    http: reqwest::Client,
    state: Arc<Mutex<State>>,
}

fn free_port() -> std::io::Result<u16> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

impl PotServer {
    pub fn new(cfg: PotServerConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .no_proxy()
            .build()
            .expect("cliente HTTP local");
        Self {
            cfg,
            http,
            state: Arc::new(Mutex::new(State {
                child: None,
                port: 0,
                last_used: Instant::now(),
                deaths: 0,
                given_up: false,
                watcher: None,
            })),
        }
    }

    /// Garante o servidor de pé e devolve a porta. Reutiliza o processo vivo; se ele morreu,
    /// reinicia uma vez; na 2ª morte desiste (o chamador segue sem token).
    pub async fn ensure_running(&self) -> CoreResult<u16> {
        let mut st = self.state.lock().await;
        if st.given_up {
            return Err(Self::unavailable());
        }
        if let Some(child) = st.child.as_mut() {
            if matches!(child.try_wait(), Ok(None)) {
                st.last_used = Instant::now();
                return Ok(st.port);
            }
            st.child = None;
            st.deaths += 1;
            tracing::warn!(deaths = st.deaths, "o servidor de PO token morreu");
        }
        loop {
            if st.deaths >= 2 {
                st.given_up = true;
                return Err(Self::unavailable());
            }
            match self.start(&mut st).await {
                Ok(port) => return Ok(port),
                Err(e) => {
                    st.deaths += 1;
                    tracing::warn!(error = %e, deaths = st.deaths, "falha ao iniciar o servidor de PO token");
                }
            }
        }
    }

    fn unavailable() -> CoreError {
        CoreError::coded(
            "pot_unavailable",
            "o servidor de PO token não está disponível; seguindo sem token",
        )
    }

    async fn start(&self, st: &mut State) -> CoreResult<u16> {
        let port = free_port()?;
        let mut command = tool_command(&self.cfg.program, (self.cfg.args)(port));
        {
            let cmd = command.command_mut();
            match self.open_log() {
                Some(file) => {
                    let stderr = file.try_clone()?;
                    cmd.stdout(file).stderr(stderr);
                }
                None => {
                    cmd.stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null());
                }
            }
            if let Some(cwd) = &self.cfg.cwd {
                cmd.current_dir(cwd);
            }
        }
        let mut child = spawn_tool(command)?;

        let deadline = Instant::now() + self.cfg.ping_timeout;
        let url = format!("http://127.0.0.1:{port}/ping");
        loop {
            if let Ok(Some(status)) = child.try_wait() {
                return Err(CoreError::coded(
                    "pot_start_failed",
                    format!("o servidor encerrou ao iniciar ({status})"),
                ));
            }
            if matches!(self.http.get(&url).send().await, Ok(r) if r.status().is_success()) {
                break;
            }
            if Instant::now() >= deadline {
                let _ = kill_tree(child.as_mut()).await;
                return Err(CoreError::coded(
                    "pot_start_failed",
                    "o servidor não respondeu ao /ping a tempo",
                ));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        st.child = Some(child);
        st.port = port;
        st.last_used = Instant::now();
        if let Some(old) = st.watcher.take() {
            old.abort();
        }
        st.watcher = Some(self.spawn_idle_watcher());
        Ok(port)
    }

    fn open_log(&self) -> Option<std::fs::File> {
        let path = self.cfg.log_file.as_ref()?;
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
    }

    /// Encerra o servidor após `idle_timeout` sem uso.
    fn spawn_idle_watcher(&self) -> JoinHandle<()> {
        let state = Arc::clone(&self.state);
        let (idle_timeout, idle_check) = (self.cfg.idle_timeout, self.cfg.idle_check);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(idle_check).await;
                let mut st = state.lock().await;
                if st.child.is_none() {
                    break;
                }
                if st.last_used.elapsed() >= idle_timeout {
                    if let Some(mut child) = st.child.take() {
                        let _ = kill_tree(child.as_mut()).await;
                    }
                    st.deaths = 0;
                    st.watcher = None;
                    tracing::info!("servidor de PO token encerrado por ociosidade");
                    break;
                }
            }
        })
    }

    /// Encerra o servidor (fim do app ou troca de versão do bgutil).
    pub async fn stop(&self) {
        let mut st = self.state.lock().await;
        if let Some(watcher) = st.watcher.take() {
            watcher.abort();
        }
        if let Some(mut child) = st.child.take() {
            let _ = kill_tree(child.as_mut()).await;
        }
        st.deaths = 0;
        st.given_up = false;
    }

    pub async fn is_running(&self) -> bool {
        let mut st = self.state.lock().await;
        match st.child.as_mut() {
            Some(child) => matches!(child.try_wait(), Ok(None)),
            None => false,
        }
    }

    /// PID do processo principal (diagnóstico e testes).
    pub async fn pid(&self) -> Option<u32> {
        self.state.lock().await.child.as_ref().and_then(|c| c.id())
    }

    pub async fn gave_up(&self) -> bool {
        self.state.lock().await.given_up
    }
}

#[cfg(test)]
mod tests;

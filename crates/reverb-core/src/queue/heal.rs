//! Autocura do yt-dlp (arquitetura §9): quando o YouTube muda, atualiza o yt-dlp, escala para o
//! canal nightly e, antes de tudo, liga o provedor de PO token. Uma única autocura por vez e no
//! máximo uma por hora; vários jobs falhando juntos compartilham o mesmo resultado.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::db::Db;
use crate::error::CoreResult;
use crate::events::EventSink;
use crate::settings::{PotProvider, SettingsPatch, SettingsService, YtdlpChannel};
use crate::tools::{InstallOutcome, Tool, ToolsManager};
use crate::ytdlp::errors::ErrorKind;

pub const EVENT_HEAL: &str = "heal://state";
pub const EVENT_NOTICE: &str = "notice";
const KV_LAST_AT: &str = "heal_last_at";
const KV_LEVEL: &str = "heal_level";
const KV_NIGHTLY: &str = "heal_switched_to_nightly";
const HOUR_SECS: u64 = 3600;

/// O que a autocura precisa do gerenciador de ferramentas (trait para usar mocks nos testes).
#[async_trait]
pub trait HealTools: Send + Sync {
    /// Atualiza o yt-dlp no canal atual. `true` = instalou uma versão nova.
    async fn update_ytdlp(&self) -> CoreResult<bool>;
    /// Persiste `ytdlpChannel = nightly` e instala o yt-dlp nightly.
    async fn switch_to_nightly(&self) -> CoreResult<()>;
    fn channel(&self) -> YtdlpChannel;
    fn pot_provider(&self) -> PotProvider;
    /// O provedor de PO token já está valendo (ligado agora ou por `always`)?
    async fn pot_active(&self) -> bool;
    /// Liga o provedor por 24 h (instalando o bgutil se faltar) e sobe o servidor local.
    async fn enable_pot(&self) -> CoreResult<()>;
}

/// Implementação real sobre o `ToolsManager`.
pub struct ToolsHeal {
    tools: Arc<ToolsManager>,
    settings: Arc<SettingsService>,
}

impl ToolsHeal {
    pub fn new(tools: Arc<ToolsManager>, settings: Arc<SettingsService>) -> Self {
        Self { tools, settings }
    }
}

#[async_trait]
impl HealTools for ToolsHeal {
    async fn update_ytdlp(&self) -> CoreResult<bool> {
        Ok(matches!(
            self.tools.update(Tool::Ytdlp).await?,
            InstallOutcome::Installed { .. }
        ))
    }

    async fn switch_to_nightly(&self) -> CoreResult<()> {
        let patch: SettingsPatch = serde_json::from_value(json!({ "ytdlpChannel": "nightly" }))?;
        self.settings.update(patch).await?;
        self.tools.update(Tool::Ytdlp).await?;
        Ok(())
    }

    fn channel(&self) -> YtdlpChannel {
        self.settings.get().ytdlp_channel
    }

    fn pot_provider(&self) -> PotProvider {
        self.settings.get().pot_provider
    }

    async fn pot_active(&self) -> bool {
        self.tools
            .pot_policy()
            .is_active(self.pot_provider())
            .await
            .unwrap_or(false)
    }

    async fn enable_pot(&self) -> CoreResult<()> {
        self.tools.pot_policy().enable_auto().await?;
        if self.tools.bgutil_paths().is_none() {
            self.tools.install(Tool::Bgutil).await?;
        }
        // `pot_args` sobe o servidor local; sem ele o provedor não funciona.
        match self.tools.pot_args().await {
            Some(_) => Ok(()),
            None => Err(crate::CoreError::coded(
                "pot_unavailable",
                "o servidor de PO token não subiu",
            )),
        }
    }
}

/// Decisão da autocura para um job que falhou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealOutcome {
    /// Reenfileirar sem consumir tentativa.
    Requeue,
    /// Falha permanente com o erro original (ex.: `bot_check` sem o que tentar).
    Fail,
    /// Autocura esgotada: `errors.extractorPersistent`.
    FailPersistent,
}

type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;

pub struct HealCoordinator {
    tools: Arc<dyn HealTools>,
    db: Db,
    sink: Arc<dyn EventSink>,
    gate: tokio::sync::Mutex<()>,
    /// Sobe a cada autocura que mudou algo (PO token, atualização, nightly). Jobs que começaram
    /// antes e falharam só são reenfileirados: o conserto já aconteceu enquanto rodavam.
    epoch: AtomicU64,
    active: AtomicUsize,
    clock: Clock,
}

impl HealCoordinator {
    pub fn new(tools: Arc<dyn HealTools>, db: Db, sink: Arc<dyn EventSink>) -> Self {
        Self::with_clock(tools, db, sink, Arc::new(crate::tools::github::now_secs))
    }

    /// Relógio injetável (segundos Unix), para testar o limite de 1 hora.
    pub fn with_clock(
        tools: Arc<dyn HealTools>,
        db: Db,
        sink: Arc<dyn EventSink>,
        clock: Clock,
    ) -> Self {
        Self {
            tools,
            db,
            sink,
            gate: tokio::sync::Mutex::new(()),
            epoch: AtomicU64::new(0),
            active: AtomicUsize::new(0),
            clock,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::SeqCst)
    }

    /// Há uma autocura em andamento (a fila não inicia jobs novos enquanto isso).
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst) > 0
    }

    fn state(&self, stage: &str) {
        self.sink.emit(EVENT_HEAL, json!({ "stage": stage }));
    }

    fn notice(&self, level: &str, key: &str) {
        self.sink.emit(
            EVENT_NOTICE,
            json!({ "level": level, "i18nKey": key, "params": {} }),
        );
    }

    fn bump(&self) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
    }

    /// Trata uma falha `extractor` ou `bot_check`. `started_epoch` é o `epoch()` de quando o
    /// job começou a rodar.
    pub async fn handle(&self, kind: ErrorKind, stderr: &str, started_epoch: u64) -> HealOutcome {
        if self.epoch() > started_epoch {
            return HealOutcome::Requeue;
        }
        let _gate = self.gate.lock().await;
        if self.epoch() > started_epoch {
            return HealOutcome::Requeue;
        }
        self.active.fetch_add(1, Ordering::SeqCst);
        let outcome = self.run(kind, stderr).await;
        self.active.fetch_sub(1, Ordering::SeqCst);
        outcome
    }

    async fn run(&self, kind: ErrorKind, stderr: &str) -> HealOutcome {
        let forbidden =
            kind == ErrorKind::Extractor && stderr.to_lowercase().contains("http error 403");

        // Passo 0 — PO token.
        if (kind == ErrorKind::BotCheck || forbidden)
            && self.tools.pot_provider() == PotProvider::Auto
            && !self.tools.pot_active().await
        {
            self.state("enabling_pot");
            match self.tools.enable_pot().await {
                Ok(()) => {
                    self.bump();
                    self.notice("info", "notices.potEnabled");
                    self.state("done");
                    return HealOutcome::Requeue;
                }
                Err(error) => tracing::warn!(%error, "não foi possível ligar o PO token"),
            }
        }
        if kind == ErrorKind::BotCheck {
            self.state("failed");
            return HealOutcome::Fail;
        }

        // Passos 1–4 — yt-dlp.
        let now = (self.clock)();
        let last_at: Option<u64> = self.kv(KV_LAST_AT).await.and_then(|t| t.parse().ok());
        let level: u32 = self
            .kv(KV_LEVEL)
            .await
            .and_then(|t| t.parse().ok())
            .unwrap_or(0);
        let within_hour = last_at.is_some_and(|t| now < t + HOUR_SECS);
        let escalate = within_hour && level == 1 && self.tools.channel() == YtdlpChannel::Stable;
        if within_hour && !escalate {
            // Já tentamos tudo há menos de 1 h (passo 5): sem nova autocura.
            self.state("failed");
            return HealOutcome::FailPersistent;
        }

        self.state("checking");
        if !escalate {
            match self.tools.update_ytdlp().await {
                Ok(true) => {
                    self.record(now, 1).await;
                    self.bump();
                    self.state("done");
                    return HealOutcome::Requeue;
                }
                Ok(false) => {}
                Err(error) => {
                    tracing::warn!(%error, "autocura: falha ao atualizar o yt-dlp");
                    self.state("failed");
                    return HealOutcome::FailPersistent;
                }
            }
            if self.tools.channel() != YtdlpChannel::Stable {
                self.record(now, 2).await;
                self.state("failed");
                return HealOutcome::FailPersistent;
            }
        }

        // Passo 3 — canal nightly.
        match self.tools.switch_to_nightly().await {
            Ok(()) => {
                self.record(now, 2).await;
                let _ = self.db.kv_set(KV_NIGHTLY, &now.to_string()).await;
                self.bump();
                self.notice("warning", "notices.ytdlpNightly");
                self.state("done");
                HealOutcome::Requeue
            }
            Err(error) => {
                tracing::warn!(%error, "autocura: falha ao trocar para o nightly");
                self.state("failed");
                HealOutcome::FailPersistent
            }
        }
    }

    async fn kv(&self, key: &str) -> Option<String> {
        self.db.kv_get(key).await.ok().flatten()
    }

    async fn record(&self, at: u64, level: u32) {
        let _ = self.db.kv_set(KV_LAST_AT, &at.to_string()).await;
        let _ = self.db.kv_set(KV_LEVEL, &level.to_string()).await;
    }
}

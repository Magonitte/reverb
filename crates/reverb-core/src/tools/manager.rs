//! `ToolsManager`: instala, verifica, atualiza e reverte as ferramentas externas (arquitetura §16).

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{OwnedRwLockReadGuard, RwLock};

use super::checksum::{parse_first_hash, parse_sums_for};
use super::github::{now_secs, GithubClient, Release, DEFAULT_API_BASE};
use super::install::{
    copy_dir_all, download_to, extract, fetch_text, find_dir, find_file, make_executable,
    rename_with_retry, smoke_test,
};
use super::js_runtime::{detect_js_runtime, Detection, JsKind, JsRuntimeChoice};
use super::manifest::{now_rfc3339, Installed, Manifest};
use super::pot::{pot_args, PotPolicy, PotServer, PotServerConfig};
use super::process::run_capture;
use super::spec::{spec_for, ChecksumSource, PackageKind, Platform, Tool, ToolSpec, VersionKind};
use super::types::{
    InstallOutcome, ToolChanged, ToolPhase, ToolProgress, ToolStatus, UpdateInfo, EVENT_CHANGED,
    EVENT_PROGRESS,
};
use super::version::{compare, parse_version, tag_to_version};
use crate::db::Db;
use crate::error::{CoreError, CoreResult};
use crate::events::EventSink;
use crate::settings::{PotProvider, Settings, SettingsService};

/// Guard de leitura: runners de yt-dlp/ffmpeg seguram enquanto usam as ferramentas;
/// instalar/reverter pega a escrita e espera todos soltarem.
pub type RunGuard = OwnedRwLockReadGuard<()>;

const DAY_SECS: u64 = 24 * 3600;
const YTDLP_CHECK_INTERVAL: u64 = DAY_SECS;
const OTHER_CHECK_INTERVAL: u64 = 7 * DAY_SECS;

#[derive(Debug, Clone)]
pub struct ToolsConfig {
    pub tools_dir: PathBuf,
    pub platform: Platform,
    pub github_base_url: String,
    pub github_token: Option<String>,
    /// Variáveis de ambiente extras nos testes de fumaça (os testes usam
    /// `FAKE_TOOL_VERSION` sem alterar o ambiente do processo).
    pub extra_env: Vec<(String, String)>,
}

impl ToolsConfig {
    pub fn new(tools_dir: impl Into<PathBuf>) -> Self {
        Self {
            tools_dir: tools_dir.into(),
            platform: Platform::current(),
            github_base_url: DEFAULT_API_BASE.to_string(),
            github_token: std::env::var("GITHUB_TOKEN").ok(),
            extra_env: Vec::new(),
        }
    }
}

/// Apaga a pasta ao sair do escopo (staging ou versão pela metade) a menos que seja desarmada.
struct DirGuard {
    path: PathBuf,
    armed: bool,
}

impl DirGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for DirGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

pub struct ToolsManager {
    cfg: ToolsConfig,
    http: reqwest::Client,
    github: GithubClient,
    db: Db,
    settings: Arc<SettingsService>,
    sink: Arc<dyn EventSink>,
    manifest: Mutex<Manifest>,
    run_lock: Arc<RwLock<()>>,
    install_gate: tokio::sync::Mutex<()>,
    pot_policy: PotPolicy,
    pot_server: tokio::sync::Mutex<Option<Arc<PotServer>>>,
}

fn version_label(spec: &ToolSpec, release: &Release, asset_updated_at: Option<&str>) -> String {
    match (spec.version_kind, asset_updated_at) {
        // Tag rolante (`latest`): o rótulo vem do `updated_at` do asset, sem `:` (pasta válida no Windows).
        (VersionKind::Rolling, Some(updated)) => updated.replace([':', '-'], ""),
        _ => tag_to_version(spec.tool, &release.tag_name),
    }
}

impl ToolsManager {
    pub fn new(
        cfg: ToolsConfig,
        db: Db,
        settings: Arc<SettingsService>,
        sink: Arc<dyn EventSink>,
    ) -> CoreResult<Self> {
        std::fs::create_dir_all(&cfg.tools_dir)?;
        // Sobras de instalações interrompidas.
        let _ = std::fs::remove_dir_all(cfg.tools_dir.join(".staging"));
        let http = reqwest::Client::builder()
            .user_agent(format!("Reverb/{}", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(60))
            .build()?;
        let github = GithubClient::new(
            http.clone(),
            cfg.github_base_url.clone(),
            cfg.github_token.clone(),
            Some(db.clone()),
        );
        let manifest = Manifest::load(&cfg.tools_dir);
        Ok(Self {
            pot_policy: PotPolicy::new(db.clone()),
            cfg,
            http,
            github,
            db,
            settings,
            sink,
            manifest: Mutex::new(manifest),
            run_lock: Arc::new(RwLock::new(())),
            install_gate: tokio::sync::Mutex::new(()),
            pot_server: tokio::sync::Mutex::new(None),
        })
    }

    pub fn tools_dir(&self) -> &Path {
        &self.cfg.tools_dir
    }

    pub fn pot_policy(&self) -> &PotPolicy {
        &self.pot_policy
    }

    fn spec(&self, tool: Tool) -> CoreResult<ToolSpec> {
        let channel = self.settings.get().ytdlp_channel;
        spec_for(tool, channel, self.cfg.platform).ok_or_else(|| {
            CoreError::coded(
                "unsupported_platform",
                format!("{} não tem build para esta plataforma", tool.id()),
            )
        })
    }

    fn channel_label(&self, tool: Tool) -> Option<String> {
        (tool == Tool::Ytdlp).then(|| match self.settings.get().ytdlp_channel {
            crate::settings::YtdlpChannel::Stable => "stable".to_string(),
            crate::settings::YtdlpChannel::Nightly => "nightly".to_string(),
        })
    }

    fn progress(&self, tool: Tool, phase: ToolPhase, percent: u8) {
        let payload = ToolProgress {
            tool,
            phase,
            percent,
        };
        if let Ok(value) = serde_json::to_value(payload) {
            self.sink.emit(EVENT_PROGRESS, value);
        }
    }

    fn emit_changed(&self, tool: Tool) {
        let version = self.installed(tool).map(|i| i.version);
        if let Ok(value) = serde_json::to_value(ToolChanged { tool, version }) {
            self.sink.emit(EVENT_CHANGED, value);
        }
    }

    fn installed(&self, tool: Tool) -> Option<Installed> {
        self.manifest
            .lock()
            .expect("manifesto")
            .current(tool.id())
            .cloned()
    }

    fn version_dir(&self, tool: Tool, version: &str) -> PathBuf {
        self.cfg.tools_dir.join(tool.id()).join(version)
    }

    // ------------------------------------------------------------------ consultas

    /// Caminho do binário principal da versão atual.
    pub fn resolve(&self, tool: Tool) -> CoreResult<PathBuf> {
        let spec = self.spec(tool)?;
        let main = spec.main_binary().ok_or_else(|| {
            CoreError::coded("tool_missing", format!("{} não tem binário", tool.id()))
        })?;
        self.resolve_file(tool, &main.install_as)
    }

    /// `ffprobe` vem no mesmo pacote do FFmpeg.
    pub fn resolve_ffprobe(&self) -> CoreResult<PathBuf> {
        self.resolve_file(Tool::Ffmpeg, &self.cfg.platform.exe_name("ffprobe"))
    }

    fn resolve_file(&self, tool: Tool, file: &str) -> CoreResult<PathBuf> {
        let current = self.installed(tool).ok_or_else(|| {
            CoreError::coded("tool_missing", format!("{} não está instalado", tool.id()))
        })?;
        let path = self.version_dir(tool, &current.version).join(file);
        if path.is_file() {
            Ok(path)
        } else {
            Err(CoreError::coded(
                "tool_missing",
                format!("arquivo de {} ausente: {}", tool.id(), path.display()),
            ))
        }
    }

    /// Pastas do bgutil: (`plugins/` para `--plugin-dirs`, `server/`).
    pub fn bgutil_paths(&self) -> Option<(PathBuf, PathBuf)> {
        let current = self.installed(Tool::Bgutil)?;
        let dir = self.version_dir(Tool::Bgutil, &current.version);
        let (plugins, server) = (dir.join("plugins"), dir.join("server"));
        (plugins.is_dir() && server.is_dir()).then_some((plugins, server))
    }

    /// Runners seguram este guard de leitura durante o uso do yt-dlp/ffmpeg.
    pub async fn acquire_run(&self) -> RunGuard {
        Arc::clone(&self.run_lock).read_owned().await
    }

    pub async fn status(&self) -> CoreResult<Vec<ToolStatus>> {
        let mut out = Vec::new();
        for tool in Tool::ALL {
            let entry = self
                .manifest
                .lock()
                .expect("manifesto")
                .entry(tool.id())
                .cloned();
            let current = entry.as_ref().and_then(|e| e.current.clone());
            let previous = entry.as_ref().and_then(|e| e.previous.clone());
            let info = self.stored_update_info(tool).await;
            let path = if tool == Tool::Bgutil {
                self.bgutil_paths()
                    .map(|(_, server)| server.display().to_string())
            } else {
                self.resolve(tool).ok().map(|p| p.display().to_string())
            };
            out.push(ToolStatus {
                tool,
                required: matches!(tool, Tool::Ytdlp | Tool::Ffmpeg),
                installed: current.is_some() && path.is_some(),
                version: current.as_ref().map(|c| {
                    c.reported_version
                        .clone()
                        .unwrap_or_else(|| c.version.clone())
                }),
                previous_version: previous.map(|p| p.reported_version.unwrap_or(p.version)),
                channel: current.as_ref().and_then(|c| c.channel.clone()),
                installed_at: current.as_ref().map(|c| c.installed_at.clone()),
                path,
                latest_version: info.as_ref().map(|i| i.latest.clone()),
                update_available: info.as_ref().is_some_and(|i| i.update_available),
                last_checked: info.map(|i| i.checked_at),
            });
        }
        Ok(out)
    }

    async fn stored_update_info(&self, tool: Tool) -> Option<UpdateInfo> {
        let raw = self
            .db
            .kv_get(&format!("tools.update.{}", tool.id()))
            .await
            .ok()
            .flatten()?;
        serde_json::from_str(&raw).ok()
    }

    // ------------------------------------------------------------------ instalação

    fn needs_install(
        &self,
        spec: &ToolSpec,
        current: Option<&Installed>,
        release: &Release,
        asset_updated_at: Option<&str>,
    ) -> bool {
        let Some(current) = current else {
            return true;
        };
        if current.channel != self.channel_label(spec.tool) {
            return true;
        }
        match spec.version_kind {
            VersionKind::Rolling => match (asset_updated_at, current.asset_updated_at.as_deref()) {
                (Some(latest), Some(installed)) => {
                    compare(VersionKind::Rolling, latest, installed).is_gt()
                }
                _ => true,
            },
            kind => compare(
                kind,
                &tag_to_version(spec.tool, &release.tag_name),
                &current.version,
            )
            .is_gt(),
        }
    }

    /// Instala a última versão se a atual faltar ou estiver defasada (usa o cache de 1 h do GitHub).
    pub async fn install(&self, tool: Tool) -> CoreResult<InstallOutcome> {
        self.ensure_latest(tool, false).await
    }

    /// Como `install`, mas consulta o GitHub sem cache. Troca de canal do yt-dlp reinstala.
    pub async fn update(&self, tool: Tool) -> CoreResult<InstallOutcome> {
        self.ensure_latest(tool, true).await
    }

    async fn ensure_latest(&self, tool: Tool, force_api: bool) -> CoreResult<InstallOutcome> {
        let spec = self.spec(tool)?;
        let release = self.github.latest(spec.repo, force_api).await?;
        let asset = release
            .assets
            .iter()
            .find(|a| spec.asset_matcher().is_match(&a.name))
            .cloned()
            .ok_or_else(|| {
                CoreError::coded(
                    "asset_not_found",
                    format!(
                        "nenhum arquivo de {} combina com {} no release {}",
                        tool.id(),
                        spec.asset_regex,
                        release.tag_name
                    ),
                )
            })?;
        let current = self.installed(tool);
        if !self.needs_install(
            &spec,
            current.as_ref(),
            &release,
            asset.updated_at.as_deref(),
        ) {
            let version = current.map(|c| c.version).unwrap_or_default();
            return Ok(InstallOutcome::UpToDate { version });
        }
        if tool == Tool::Bgutil {
            // O servidor roda no Deno gerenciado, mesmo que o yt-dlp use o Node do sistema.
            if self.resolve(Tool::Deno).is_err() {
                Box::pin(self.ensure_latest(Tool::Deno, force_api)).await?;
            }
            return self.install_bgutil(&spec, &release, &asset).await;
        }
        self.install_release(&spec, &release, &asset).await
    }

    async fn verify_checksum(
        &self,
        spec: &ToolSpec,
        release: &Release,
        asset_name: &str,
        actual: &str,
    ) -> CoreResult<()> {
        let missing = |what: &str| {
            CoreError::coded(
                "checksum_missing",
                format!("o release não traz o arquivo de checksum {what}"),
            )
        };
        let expected = match spec.checksum {
            ChecksumSource::None => {
                tracing::warn!(
                    tool = spec.tool.id(),
                    "fonte sem checksum publicado; instalando sem verificar o hash"
                );
                return Ok(());
            }
            ChecksumSource::SumsFile(name) => {
                let file = release.asset(name).ok_or_else(|| missing(name))?;
                let text = fetch_text(&self.http, &file.browser_download_url).await?;
                parse_sums_for(&text, asset_name).ok_or_else(|| {
                    CoreError::coded("checksum_missing", format!("{name} não lista {asset_name}"))
                })?
            }
            ChecksumSource::PerAsset(suffix) => {
                let name = format!("{asset_name}{suffix}");
                let file = release.asset(&name).ok_or_else(|| missing(&name))?;
                let text = fetch_text(&self.http, &file.browser_download_url).await?;
                parse_first_hash(&text).ok_or_else(|| {
                    CoreError::coded("checksum_missing", format!("{name} não contém um SHA-256"))
                })?
            }
        };
        if expected.eq_ignore_ascii_case(actual) {
            Ok(())
        } else {
            Err(CoreError::coded(
                "checksum_mismatch",
                format!("SHA-256 de {asset_name} diverge (esperado {expected}, obtido {actual})"),
            ))
        }
    }

    async fn install_release(
        &self,
        spec: &ToolSpec,
        release: &Release,
        asset: &super::github::Asset,
    ) -> CoreResult<InstallOutcome> {
        let tool = spec.tool;
        let _gate = self.install_gate.lock().await;
        let label = version_label(spec, release, asset.updated_at.as_deref());

        let stage_root = self.cfg.tools_dir.join(".staging");
        let stage = stage_root.join(format!("{}-{}", tool.id(), unique_suffix()));
        std::fs::create_dir_all(&stage)?;
        let _stage_guard = DirGuard::new(stage.clone());

        // 1. baixar
        let archive = stage.join(&asset.name);
        self.progress(tool, ToolPhase::Downloading, 0);
        let sha = download_to(&self.http, &asset.browser_download_url, &archive, |p| {
            self.progress(tool, ToolPhase::Downloading, p)
        })
        .await?;

        // 2. verificar (divergência ⇒ erro e nada é trocado)
        self.progress(tool, ToolPhase::Verifying, 0);
        self.verify_checksum(spec, release, &asset.name, &sha)
            .await?;
        self.progress(tool, ToolPhase::Verifying, 100);

        // 3. extrair e separar os binários
        self.progress(tool, ToolPhase::Extracting, 0);
        let out = stage.join("out");
        std::fs::create_dir_all(&out)?;
        if spec.package == PackageKind::Exe {
            let main = spec
                .main_binary()
                .ok_or_else(|| CoreError::Internal("spec sem binário".into()))?;
            std::fs::copy(&archive, out.join(&main.install_as))?;
        } else {
            let extracted = stage.join("extracted");
            extract(spec.package, &archive, &extracted).await?;
            for binary in &spec.binaries {
                let found = find_file(&extracted, &binary.find).ok_or_else(|| {
                    CoreError::coded(
                        "binary_not_found",
                        format!("{} não encontrado dentro de {}", binary.find, asset.name),
                    )
                })?;
                std::fs::copy(found, out.join(&binary.install_as))?;
            }
        }
        for binary in &spec.binaries {
            make_executable(&out.join(&binary.install_as))?;
        }
        self.progress(tool, ToolPhase::Extracting, 100);

        // 4. teste de fumaça
        self.progress(tool, ToolPhase::Testing, 0);
        let mut reported = None;
        for (index, binary) in spec.binaries.iter().enumerate() {
            let output = smoke_test(
                &out.join(&binary.install_as),
                spec.version_args,
                &self.cfg.extra_env,
            )
            .await?;
            if index == 0 {
                reported = parse_version(spec, &output);
            }
        }
        let reported = reported.ok_or_else(|| {
            CoreError::coded(
                "smoke_test_failed",
                format!("não foi possível ler a versão de {}", tool.id()),
            )
        })?;
        if spec.version_kind != VersionKind::Rolling
            && compare(spec.version_kind, &reported, &label).is_ne()
        {
            tracing::warn!(
                tool = tool.id(),
                reported,
                label,
                "versão do binário difere da tag"
            );
        }
        self.progress(tool, ToolPhase::Testing, 100);

        // 5. colocar no lugar e trocar o manifesto
        let final_dir = self.version_dir(tool, &label);
        if final_dir.exists() {
            std::fs::remove_dir_all(&final_dir)?;
        }
        if let Some(parent) = final_dir.parent() {
            std::fs::create_dir_all(parent)?;
        }
        rename_with_retry(&out, &final_dir).await?;
        let installed = Installed {
            version: label.clone(),
            reported_version: Some(reported),
            channel: self.channel_label(tool),
            installed_at: now_rfc3339(),
            asset_updated_at: asset.updated_at.clone(),
        };
        let previous = self.commit(tool, installed).await?;
        Ok(InstallOutcome::Installed {
            version: label,
            previous,
        })
    }

    /// Instalação do bgutil (E4): plugin intacto + código do servidor + `deno install`.
    async fn install_bgutil(
        &self,
        spec: &ToolSpec,
        release: &Release,
        asset: &super::github::Asset,
    ) -> CoreResult<InstallOutcome> {
        let tool = Tool::Bgutil;
        let zipball = release.zipball_url.clone().ok_or_else(|| {
            CoreError::coded(
                "asset_not_found",
                "o release não traz o código-fonte (zipball)",
            )
        })?;
        tracing::warn!("bgutil não publica checksum; instalando sem verificar o hash");
        let deno = self.resolve(Tool::Deno)?;
        let _gate = self.install_gate.lock().await;
        let version = tag_to_version(tool, &release.tag_name);

        let stage = self
            .cfg
            .tools_dir
            .join(".staging")
            .join(format!("bgutil-{}", unique_suffix()));
        std::fs::create_dir_all(&stage)?;
        let _stage_guard = DirGuard::new(stage.clone());

        let final_dir = self.version_dir(tool, &version);
        if final_dir.exists() {
            std::fs::remove_dir_all(&final_dir)?;
        }
        std::fs::create_dir_all(final_dir.join("plugins"))?;
        let mut final_guard = DirGuard::new(final_dir.clone());

        // plugin: o zip fica intacto dentro de plugins/ (usado via --plugin-dirs)
        self.progress(tool, ToolPhase::Downloading, 0);
        download_to(
            &self.http,
            &asset.browser_download_url,
            &final_dir.join("plugins").join(&asset.name),
            |p| self.progress(tool, ToolPhase::Downloading, p / 2),
        )
        .await?;

        // código do servidor: zipball da mesma tag ⇒ pasta server/
        let zip_path = stage.join("server.zip");
        download_to(&self.http, &zipball, &zip_path, |p| {
            self.progress(tool, ToolPhase::Downloading, 50 + p / 2)
        })
        .await?;
        self.progress(tool, ToolPhase::Extracting, 0);
        let extracted = stage.join("src");
        extract(PackageKind::Zip, &zip_path, &extracted).await?;
        let server_src = find_dir(&extracted, "server").ok_or_else(|| {
            CoreError::coded(
                "binary_not_found",
                "pasta server/ ausente no código do bgutil",
            )
        })?;
        copy_dir_all(&server_src, &final_dir.join("server"))?;
        self.progress(tool, ToolPhase::Extracting, 100);

        // dependências com o Deno gerenciado
        self.progress(tool, ToolPhase::InstallingDeps, 0);
        let server_dir = final_dir.join("server");
        let output = run_capture(
            &deno,
            ["install", "--allow-scripts=npm:canvas", "--frozen"],
            |command| {
                command.current_dir(&server_dir);
            },
            Duration::from_secs(300),
        )
        .await
        .map_err(|e| CoreError::coded("deps_install_failed", format!("deno install: {e}")))?;
        if !output.success {
            return Err(CoreError::coded(
                "deps_install_failed",
                format!("deno install falhou: {}", output.stderr.trim()),
            ));
        }
        self.progress(tool, ToolPhase::InstallingDeps, 100);

        // teste de fumaça: subir o servidor, GET /ping, derrubar
        self.progress(tool, ToolPhase::Testing, 0);
        let server = PotServer::new(bgutil_server_config(deno, &server_dir));
        let started = server.ensure_running().await;
        server.stop().await;
        started?;
        self.progress(tool, ToolPhase::Testing, 100);

        let installed = Installed {
            version: version.clone(),
            reported_version: None,
            channel: None,
            installed_at: now_rfc3339(),
            asset_updated_at: asset.updated_at.clone(),
        };
        let previous = self.commit(tool, installed).await?;
        final_guard.disarm();
        let _ = spec; // a spec do bgutil só descreve a origem (repo/asset)
        self.reset_pot_server().await;
        Ok(InstallOutcome::Installed { version, previous })
    }

    /// Troca o manifesto sob o lock de escrita (espera os jobs soltarem as ferramentas),
    /// remove versões além de current+previous e emite `tools://changed`.
    /// Devolve a versão anterior, se houver.
    async fn commit(&self, tool: Tool, installed: Installed) -> CoreResult<Option<String>> {
        let guard = self.write_lock(tool).await;
        let previous = {
            let mut manifest = self.manifest.lock().expect("manifesto");
            let entry = manifest.tools.entry(tool.id().to_string()).or_default();
            let old = entry.current.take();
            if let Some(old) = old {
                if old.version != installed.version {
                    entry.previous = Some(old);
                }
            }
            entry.current = Some(installed);
            let previous = entry.previous.as_ref().map(|p| p.version.clone());
            manifest.save(&self.cfg.tools_dir)?;
            previous
        };
        self.cleanup_old_versions(tool);
        drop(guard);
        self.emit_changed(tool);
        Ok(previous)
    }

    async fn write_lock(&self, tool: Tool) -> tokio::sync::OwnedRwLockWriteGuard<()> {
        match Arc::clone(&self.run_lock).try_write_owned() {
            Ok(guard) => guard,
            Err(_) => {
                self.progress(tool, ToolPhase::WaitingJobs, 0);
                Arc::clone(&self.run_lock).write_owned().await
            }
        }
    }

    fn cleanup_old_versions(&self, tool: Tool) {
        let keep: Vec<String> = {
            let manifest = self.manifest.lock().expect("manifesto");
            manifest
                .entry(tool.id())
                .map(|e| {
                    [e.current.as_ref(), e.previous.as_ref()]
                        .into_iter()
                        .flatten()
                        .map(|i| i.version.clone())
                        .collect()
                })
                .unwrap_or_default()
        };
        let Ok(entries) = std::fs::read_dir(self.cfg.tools_dir.join(tool.id())) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !keep.contains(&name) {
                if let Err(e) = std::fs::remove_dir_all(entry.path()) {
                    tracing::warn!(tool = tool.id(), version = name, error = %e, "não foi possível apagar versão antiga");
                }
            }
        }
    }

    /// Volta para a versão anterior (troca current ↔ previous).
    pub async fn rollback(&self, tool: Tool) -> CoreResult<String> {
        let _gate = self.install_gate.lock().await;
        let guard = self.write_lock(tool).await;
        let version = {
            let mut manifest = self.manifest.lock().expect("manifesto");
            let entry = manifest.tools.entry(tool.id().to_string()).or_default();
            let previous = entry.previous.clone().ok_or_else(|| {
                CoreError::coded(
                    "no_previous_version",
                    format!("{} não tem versão anterior", tool.id()),
                )
            })?;
            if !self.version_dir(tool, &previous.version).is_dir() {
                return Err(CoreError::coded(
                    "no_previous_version",
                    format!(
                        "a pasta da versão anterior de {} não existe mais",
                        tool.id()
                    ),
                ));
            }
            entry.previous = entry.current.take();
            let version = previous.version.clone();
            entry.current = Some(previous);
            manifest.save(&self.cfg.tools_dir)?;
            version
        };
        drop(guard);
        if tool == Tool::Bgutil {
            self.reset_pot_server().await;
        }
        self.emit_changed(tool);
        Ok(version)
    }

    // ------------------------------------------------------------------ pacotes em lote

    fn needs_managed_by_default(settings: &Settings) -> bool {
        settings.pot_provider == PotProvider::Always
    }

    /// Instala o que faltar entre as ferramentas obrigatórias (yt-dlp, FFmpeg, runtime JS).
    /// Devolve as ferramentas instaladas agora. Continua após erros e devolve o primeiro.
    pub async fn install_missing(&self) -> CoreResult<Vec<Tool>> {
        let path_var = std::env::var_os("PATH").unwrap_or_default();
        self.install_missing_with_path(&path_var).await
    }

    pub async fn install_missing_with_path(&self, path_var: &OsStr) -> CoreResult<Vec<Tool>> {
        let before: Vec<Tool> = Tool::ALL
            .into_iter()
            .filter(|t| self.installed(*t).is_some())
            .collect();
        let mut first_error = None;
        let mut note = |result: CoreResult<()>| {
            if let Err(e) = result {
                tracing::warn!(error = %e, "falha ao instalar ferramenta ausente");
                first_error.get_or_insert(e);
            }
        };
        for tool in [Tool::Ytdlp, Tool::Ffmpeg] {
            if self.resolve(tool).is_err() {
                note(self.install(tool).await.map(|_| ()));
            }
        }
        note(self.resolve_js_runtime(path_var).await.map(|_| ()));
        if Self::needs_managed_by_default(&self.settings.get())
            && self.installed(Tool::Bgutil).is_none()
        {
            note(self.install(Tool::Bgutil).await.map(|_| ()));
        }
        let installed_now: Vec<Tool> = Tool::ALL
            .into_iter()
            .filter(|t| self.installed(*t).is_some() && !before.contains(t))
            .collect();
        match first_error {
            Some(e) if installed_now.is_empty() => Err(e),
            _ => Ok(installed_now),
        }
    }

    // ------------------------------------------------------------------ atualizações

    /// Consulta o GitHub para cada ferramenta instalada e guarda o resultado em `kv`.
    pub async fn check_updates(&self, force: bool) -> CoreResult<Vec<UpdateInfo>> {
        let mut infos = Vec::new();
        let mut last_error = None;
        for tool in Tool::ALL {
            if self.installed(tool).is_none() {
                continue;
            }
            match self.check_one(tool, force).await {
                Ok(info) => infos.push(info),
                Err(e) => {
                    tracing::warn!(tool = tool.id(), error = %e, "falha ao verificar atualização");
                    last_error = Some(e);
                }
            }
        }
        match last_error {
            Some(e) if infos.is_empty() => Err(e),
            _ => Ok(infos),
        }
    }

    async fn check_one(&self, tool: Tool, force: bool) -> CoreResult<UpdateInfo> {
        let spec = self.spec(tool)?;
        let release = self.github.latest(spec.repo, force).await?;
        let asset = release
            .assets
            .iter()
            .find(|a| spec.asset_matcher().is_match(&a.name));
        let current = self.installed(tool);
        let update_available = asset.is_some()
            && self.needs_install(
                &spec,
                current.as_ref(),
                &release,
                asset.and_then(|a| a.updated_at.as_deref()),
            );
        let info = UpdateInfo {
            tool,
            current: current.map(|c| c.reported_version.unwrap_or(c.version)),
            latest: version_label(&spec, &release, asset.and_then(|a| a.updated_at.as_deref())),
            update_available,
            checked_at: now_rfc3339(),
        };
        self.db
            .kv_set(
                &format!("tools.update.{}", tool.id()),
                &serde_json::to_string(&info)?,
            )
            .await?;
        self.db
            .kv_set(
                &format!("tools.last_check.{}", tool.id()),
                &now_secs().to_string(),
            )
            .await?;
        Ok(info)
    }

    /// Verificações e instalações automáticas da inicialização (arquitetura §16): ferramentas
    /// faltantes e, com `autoUpdateTools`, yt-dlp 1×/dia e as demais 1×/semana.
    pub async fn background_startup(self: Arc<Self>) {
        if let Err(e) = self.install_missing().await {
            tracing::warn!(error = %e, "instalação das ferramentas ausentes falhou");
        }
        if !self.settings.get().auto_update_tools {
            return;
        }
        let now = now_secs();
        for tool in Tool::ALL {
            if self.installed(tool).is_none() {
                continue;
            }
            let interval = if tool == Tool::Ytdlp {
                YTDLP_CHECK_INTERVAL
            } else {
                OTHER_CHECK_INTERVAL
            };
            let last = self
                .db
                .kv_get(&format!("tools.last_check.{}", tool.id()))
                .await
                .ok()
                .flatten()
                .and_then(|text| text.parse::<u64>().ok());
            if !check_due(last, now, interval) {
                continue;
            }
            match self.update(tool).await {
                Ok(outcome) => {
                    tracing::info!(
                        tool = tool.id(),
                        ?outcome,
                        "verificação automática concluída"
                    );
                    if let Err(e) = self.check_one(tool, false).await {
                        tracing::warn!(tool = tool.id(), error = %e, "falha ao registrar a verificação");
                    }
                }
                Err(e) => {
                    tracing::warn!(tool = tool.id(), error = %e, "atualização automática falhou")
                }
            }
        }
    }

    // ------------------------------------------------------------------ runtime JS e PO token

    /// Escolhe o runtime JS (instalando o Deno gerenciado se for preciso).
    /// `path_var` é o valor de `PATH` a usar (o app passa `std::env::var_os("PATH")`).
    pub async fn resolve_js_runtime(&self, path_var: &OsStr) -> CoreResult<JsRuntimeChoice> {
        let managed = self.resolve(Tool::Deno).ok();
        match detect_js_runtime(self.settings.get().js_runtime, path_var, managed).await? {
            Detection::Found(choice) => Ok(choice),
            Detection::NeedManagedDeno => {
                self.install(Tool::Deno).await?;
                Ok(JsRuntimeChoice {
                    kind: JsKind::Deno,
                    path: self.resolve(Tool::Deno)?,
                })
            }
        }
    }

    /// Argumentos extras do yt-dlp quando o provedor de PO token está ativo (E4); `None` caso contrário.
    pub async fn pot_args(&self) -> Option<Vec<String>> {
        let provider = self.settings.get().pot_provider;
        if !self.pot_policy.is_active(provider).await.unwrap_or(false) {
            return None;
        }
        let Some((plugins, server_dir)) = self.bgutil_paths() else {
            tracing::warn!("provedor de PO token ativo, mas o bgutil não está instalado");
            return None;
        };
        let deno = self.resolve(Tool::Deno).ok()?;
        let server = {
            let mut slot = self.pot_server.lock().await;
            Arc::clone(slot.get_or_insert_with(|| {
                Arc::new(PotServer::new(bgutil_server_config(deno, &server_dir)))
            }))
        };
        match server.ensure_running().await {
            Ok(port) => Some(pot_args(&plugins, port)),
            Err(e) => {
                tracing::warn!(error = %e, "seguindo sem PO token");
                None
            }
        }
    }

    async fn reset_pot_server(&self) {
        let server = self.pot_server.lock().await.take();
        if let Some(server) = server {
            server.stop().await;
        }
    }

    /// Encerra processos auxiliares (servidor de PO token) ao fechar o app.
    pub async fn shutdown(&self) {
        self.reset_pot_server().await;
    }
}

/// Caminho canônico sem o prefixo `\\?\` do Windows. O Deno compara `--allow-read=.` com o caminho
/// REAL dos arquivos; com um diretório em formato curto (`JEANCA~1`) o servidor do bgutil
/// morre com `NotCapable` ao ler o `node_modules`.
fn normalize_dir(path: &Path) -> PathBuf {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let text = canonical.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with(r"UNC\") => PathBuf::from(rest),
        _ => canonical,
    }
}

/// Servidor HTTP do bgutil (E4): `deno run … ../src/main.ts --port <porta>` dentro de `server/node_modules`.
pub fn bgutil_server_config(deno: PathBuf, server_dir: &Path) -> PotServerConfig {
    let server_dir = normalize_dir(server_dir);
    let args = Arc::new(|port: u16| -> Vec<OsString> {
        [
            "run",
            "--allow-env",
            "--allow-net",
            "--allow-ffi=.",
            "--allow-read=.",
            "../src/main.ts",
            "--port",
        ]
        .iter()
        .map(OsString::from)
        .chain(std::iter::once(OsString::from(port.to_string())))
        .collect()
    });
    let mut config = PotServerConfig::new(deno, args, Some(server_dir.join("node_modules")));
    config.log_file = Some(server_dir.join("server.log"));
    config
}

/// Verificação periódica devida? (nunca verificada ⇒ sim).
pub fn check_due(last_check_secs: Option<u64>, now_secs: u64, interval_secs: u64) -> bool {
    last_check_secs.is_none_or(|last| now_secs.saturating_sub(last) >= interval_secs)
}

fn unique_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

#[cfg(test)]
mod tests;

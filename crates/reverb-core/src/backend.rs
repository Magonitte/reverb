//! `DownloadBackend`: a fronteira entre a fila/CLI e quem realmente baixa. Hoje só existe o
//! `YtDlpProcessBackend` (processo do yt-dlp); o Android (A1) terá outra implementação.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::error::{CoreError, CoreResult};
use crate::settings::{CookiesSource, SettingsService};
use crate::tools::{Tool, ToolsManager};
use crate::ytdlp::errors::{DownloadError, ErrorKind};
use crate::ytdlp::{
    Analysis, CookiesArg, DoneInfo, DownloadOptions, ProgressUpdate, SearchResult, SearchSource,
    YtDlpContext, YtDlpRunner,
};

/// O que o backend precisa saber de um job. `tmp_dir` pertence ao chamador (`JobWorkspace`):
/// o arquivo baixado fica lá até ser movido para o destino.
#[derive(Debug, Clone)]
pub struct JobSpec {
    pub url: String,
    pub tmp_dir: PathBuf,
    pub sponsorblock: Option<Vec<String>>,
}

pub type ProgressCallback<'a> = &'a (dyn Fn(ProgressUpdate) + Send + Sync);

#[async_trait]
pub trait DownloadBackend: Send + Sync {
    async fn analyze(
        &self,
        url: &str,
        cancel: &CancellationToken,
    ) -> Result<Analysis, DownloadError>;

    async fn search(
        &self,
        source: SearchSource,
        query: &str,
        limit: u32,
        cancel: &CancellationToken,
    ) -> Result<Vec<SearchResult>, DownloadError>;

    async fn download(
        &self,
        job: &JobSpec,
        on_progress: ProgressCallback<'_>,
        cancel: &CancellationToken,
    ) -> Result<DoneInfo, DownloadError>;
}

/// Monta o `YtDlpContext` de cada chamada (as configurações podem mudar entre jobs).
#[async_trait]
pub trait ContextProvider: Send + Sync {
    async fn context(&self) -> CoreResult<YtDlpContext>;
}

/// Contexto fixo (testes).
pub struct StaticContext(pub YtDlpContext);

#[async_trait]
impl ContextProvider for StaticContext {
    async fn context(&self) -> CoreResult<YtDlpContext> {
        Ok(self.0.clone())
    }
}

/// Contexto real: ferramentas do gerenciador + configurações do usuário.
pub struct ToolsContext {
    tools: Arc<ToolsManager>,
    settings: Arc<SettingsService>,
}

impl ToolsContext {
    pub fn new(tools: Arc<ToolsManager>, settings: Arc<SettingsService>) -> Self {
        Self { tools, settings }
    }
}

#[async_trait]
impl ContextProvider for ToolsContext {
    async fn context(&self) -> CoreResult<YtDlpContext> {
        let settings = self.settings.get();
        let ytdlp_path = self.tools.resolve(Tool::Ytdlp)?;
        let ffmpeg = self.tools.resolve(Tool::Ffmpeg)?;
        let ffmpeg_dir = ffmpeg
            .parent()
            .map(PathBuf::from)
            .ok_or_else(|| CoreError::coded("tool_missing", "ffmpeg sem pasta"))?;
        let path_var = std::env::var_os("PATH").unwrap_or_default();
        let js = self.tools.resolve_js_runtime(&path_var).await?;
        let cookies = match settings.cookies_source {
            CookiesSource::None => None,
            CookiesSource::Firefox => Some(CookiesArg::Browser("firefox".to_string())),
            CookiesSource::Chrome => Some(CookiesArg::Browser("chrome".to_string())),
            CookiesSource::Edge => Some(CookiesArg::Browser("edge".to_string())),
            CookiesSource::Brave => Some(CookiesArg::Browser("brave".to_string())),
            CookiesSource::File => Some(CookiesArg::File(PathBuf::from(&settings.cookies_file))),
        };
        Ok(YtDlpContext {
            ytdlp_path,
            js_runtime_arg: js.js_runtime_arg(),
            ffmpeg_dir,
            cookies,
            limit_rate_mbps: (settings.speed_limit_mbps > 0.0).then_some(settings.speed_limit_mbps),
            pot_args: self.tools.pot_args().await,
        })
    }
}

fn context_error(error: CoreError) -> DownloadError {
    let kind = match error.kind() {
        "tool_missing" => ErrorKind::Ffmpeg,
        _ => ErrorKind::Unknown,
    };
    DownloadError::new(kind, error.to_string())
}

pub struct YtDlpProcessBackend {
    runner: YtDlpRunner,
    context: Arc<dyn ContextProvider>,
}

impl YtDlpProcessBackend {
    pub fn new(runner: YtDlpRunner, context: Arc<dyn ContextProvider>) -> Self {
        Self { runner, context }
    }

    async fn context(&self) -> Result<YtDlpContext, DownloadError> {
        self.context.context().await.map_err(context_error)
    }
}

#[async_trait]
impl DownloadBackend for YtDlpProcessBackend {
    async fn analyze(
        &self,
        url: &str,
        cancel: &CancellationToken,
    ) -> Result<Analysis, DownloadError> {
        let ctx = self.context().await?;
        self.runner.analyze(&ctx, url, cancel).await
    }

    async fn search(
        &self,
        source: SearchSource,
        query: &str,
        limit: u32,
        cancel: &CancellationToken,
    ) -> Result<Vec<SearchResult>, DownloadError> {
        let ctx = self.context().await?;
        self.runner.search(&ctx, source, query, limit, cancel).await
    }

    async fn download(
        &self,
        job: &JobSpec,
        on_progress: ProgressCallback<'_>,
        cancel: &CancellationToken,
    ) -> Result<DoneInfo, DownloadError> {
        let ctx = self.context().await?;
        let options = DownloadOptions {
            url: job.url.clone(),
            tmp_dir: job.tmp_dir.clone(),
            sponsorblock: job.sponsorblock.clone(),
        };
        self.runner
            .download(&ctx, &options, cancel, on_progress)
            .await
    }
}

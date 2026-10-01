//! `YtDlpRunner`: executa o yt-dlp (analisar, buscar, baixar) com cancelamento, watchdog e
//! classificação de erros (arquitetura §8/§9).

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

use super::args::{
    analyze_collection_args, analyze_video_args, download_args, search_args, DownloadOptions,
    SearchSource, YtDlpContext,
};
use super::errors::{DownloadError, ErrorKind};
use super::models::{CollectionInfo, SearchResult, VideoInfo};
use super::parse::{parse_done_line, parse_progress_line, DoneInfo, ProgressUpdate};
use crate::exec::{run_streaming, ExecFailure, ExecOutcome, ExecSpec};
use crate::tools::ToolsManager;
use crate::urlkind::{classify, UrlKind};

/// Tempos do §8 e variáveis de ambiente extras (testes).
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Sem nenhuma saída por esse tempo no download ⇒ mata a árvore e classifica como `network`.
    pub watchdog: Duration,
    pub analyze_timeout: Duration,
    pub collection_timeout: Duration,
    pub search_timeout: Duration,
    pub extra_env: Vec<(String, String)>,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            watchdog: Duration::from_secs(300),
            analyze_timeout: Duration::from_secs(90),
            collection_timeout: Duration::from_secs(180),
            search_timeout: Duration::from_secs(90),
            extra_env: Vec::new(),
        }
    }
}

/// Resultado de `analyze`: vídeo ou coleção.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum Analysis {
    Video { info: Box<VideoInfo> },
    Collection { info: CollectionInfo },
}

pub type ProgressFn<'a> = &'a (dyn Fn(ProgressUpdate) + Send + Sync);

pub struct YtDlpRunner {
    tools: Option<Arc<ToolsManager>>,
    config: RunnerConfig,
}

impl YtDlpRunner {
    /// Com `tools`, cada execução segura o guard de leitura do gerenciador (nenhuma atualização
    /// troca o yt-dlp/ffmpeg no meio do uso).
    pub fn new(tools: Option<Arc<ToolsManager>>) -> Self {
        Self::with_config(tools, RunnerConfig::default())
    }

    pub fn with_config(tools: Option<Arc<ToolsManager>>, config: RunnerConfig) -> Self {
        Self { tools, config }
    }

    pub fn config(&self) -> &RunnerConfig {
        &self.config
    }

    async fn exec(
        &self,
        ctx: &YtDlpContext,
        args: Vec<String>,
        idle_timeout: Option<Duration>,
        total_timeout: Option<Duration>,
        cancel: &CancellationToken,
        on_stdout: &mut (dyn FnMut(&str) + Send),
    ) -> Result<ExecOutcome, DownloadError> {
        let _guard = match &self.tools {
            Some(tools) => Some(tools.acquire_run().await),
            None => None,
        };
        let mut env = vec![
            ("PYTHONIOENCODING".to_string(), "utf-8".to_string()),
            ("PYTHONUTF8".to_string(), "1".to_string()),
        ];
        env.extend(self.config.extra_env.iter().cloned());
        let spec = ExecSpec {
            program: ctx.ytdlp_path.clone(),
            args,
            env,
            idle_timeout,
            total_timeout,
        };
        let outcome = match run_streaming(spec, cancel, on_stdout).await {
            Ok(outcome) => outcome,
            Err(ExecFailure::Cancelled) => return Err(DownloadError::cancelled()),
            Err(ExecFailure::Idle) => {
                return Err(DownloadError::new(
                    ErrorKind::Network,
                    format!(
                        "o yt-dlp ficou {} s sem responder (watchdog)",
                        self.config.watchdog.as_secs()
                    ),
                ))
            }
            Err(ExecFailure::Timeout) => {
                return Err(DownloadError::new(
                    ErrorKind::Network,
                    "o yt-dlp excedeu o tempo limite",
                ))
            }
            Err(ExecFailure::Spawn(e)) => {
                return Err(DownloadError::new(
                    ErrorKind::Unknown,
                    format!("não foi possível iniciar o yt-dlp: {e}"),
                ))
            }
        };
        if outcome.success {
            Ok(outcome)
        } else {
            let code = outcome.code.map_or("?".to_string(), |c| c.to_string());
            Err(DownloadError::from_stderr(
                outcome.stderr_tail,
                &format!("o yt-dlp terminou com código {code}"),
            ))
        }
    }

    /// Roda um comando `-J` e devolve o JSON (stdout inteiro).
    async fn json(
        &self,
        ctx: &YtDlpContext,
        args: Vec<String>,
        total_timeout: Duration,
        cancel: &CancellationToken,
    ) -> Result<String, DownloadError> {
        let mut text = String::new();
        let mut collect = |line: &str| {
            text.push_str(line);
            text.push('\n');
        };
        self.exec(ctx, args, None, Some(total_timeout), cancel, &mut collect)
            .await?;
        Ok(text)
    }

    pub async fn analyze_video(
        &self,
        ctx: &YtDlpContext,
        url: &str,
        cancel: &CancellationToken,
    ) -> Result<VideoInfo, DownloadError> {
        let args = analyze_video_args(ctx, url);
        let json = self
            .json(ctx, args, self.config.analyze_timeout, cancel)
            .await?;
        VideoInfo::from_json(&json).map_err(invalid_json)
    }

    pub async fn analyze_collection(
        &self,
        ctx: &YtDlpContext,
        url: &str,
        cancel: &CancellationToken,
    ) -> Result<CollectionInfo, DownloadError> {
        let args = analyze_collection_args(ctx, url);
        let json = self
            .json(ctx, args, self.config.collection_timeout, cancel)
            .await?;
        CollectionInfo::from_json(&json).map_err(invalid_json)
    }

    /// Classifica a URL e analisa como vídeo ou coleção.
    pub async fn analyze(
        &self,
        ctx: &YtDlpContext,
        url: &str,
        cancel: &CancellationToken,
    ) -> Result<Analysis, DownloadError> {
        match classify(url) {
            UrlKind::Video { url, .. } => Ok(Analysis::Video {
                info: Box::new(self.analyze_video(ctx, &url, cancel).await?),
            }),
            UrlKind::Collection { url } => Ok(Analysis::Collection {
                info: self.analyze_collection(ctx, &url, cancel).await?,
            }),
            UrlKind::Search { .. } | UrlKind::Unsupported => Err(DownloadError::new(
                ErrorKind::Unavailable,
                format!("URL não suportada: {url}"),
            )),
        }
    }

    pub async fn search(
        &self,
        ctx: &YtDlpContext,
        source: SearchSource,
        query: &str,
        limit: u32,
        cancel: &CancellationToken,
    ) -> Result<Vec<SearchResult>, DownloadError> {
        let args = search_args(ctx, source, query, limit);
        let json = self
            .json(ctx, args, self.config.search_timeout, cancel)
            .await?;
        SearchResult::list_from_json(&json).map_err(invalid_json)
    }

    /// Baixa o melhor áudio para `options.tmp_dir`. `on_progress` recebe cada `REVERB_PROGRESS`.
    pub async fn download(
        &self,
        ctx: &YtDlpContext,
        options: &DownloadOptions,
        cancel: &CancellationToken,
        on_progress: ProgressFn<'_>,
    ) -> Result<DoneInfo, DownloadError> {
        let args = download_args(ctx, options);
        let mut done: Option<DoneInfo> = None;
        let mut on_line = |line: &str| {
            if let Some(update) = parse_progress_line(line) {
                on_progress(update);
            } else if let Some(info) = parse_done_line(line) {
                done = Some(info);
            }
        };
        let idle = Some(self.config.watchdog);
        self.exec(ctx, args, idle, None, cancel, &mut on_line)
            .await?;
        done.ok_or_else(|| {
            DownloadError::new(
                ErrorKind::Unknown,
                "o yt-dlp terminou sem informar o arquivo baixado",
            )
        })
    }
}

fn invalid_json(error: serde_json::Error) -> DownloadError {
    DownloadError::new(
        ErrorKind::Unknown,
        format!("resposta do yt-dlp não é um JSON válido: {error}"),
    )
}

#[cfg(test)]
mod tests;

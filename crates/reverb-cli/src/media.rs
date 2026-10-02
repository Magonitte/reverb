//! Comandos `analyze`, `search` e `download` (F03): usam o mesmo pipeline do app.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context, Result};
use reverb_core::backend::{DownloadBackend, ToolsContext, YtDlpProcessBackend};
use reverb_core::pipeline::{DownloadPipeline, PipelineEvent, PipelineJob};
use reverb_core::profiles::{profile, PROFILES};
use reverb_core::urlkind::{classify, UrlKind};
use reverb_core::workspace::sweep_orphans;
use reverb_core::ytdlp::errors::DownloadError;
use reverb_core::ytdlp::{Analysis, ProgressUpdate, SearchSource, YtDlpRunner};
use reverb_core::{SettingsService, Tool, ToolsManager};
use tokio_util::sync::CancellationToken;

/// Cancela com Ctrl+C: o runner mata a árvore de processos e a pasta do job é apagada.
fn cancel_on_ctrl_c() -> CancellationToken {
    let token = CancellationToken::new();
    let handle = token.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            handle.cancel();
        }
    });
    token
}

fn failure(error: DownloadError) -> anyhow::Error {
    anyhow::anyhow!("{} ({})", error.message, error.kind.as_str())
}

fn backend(manager: Arc<ToolsManager>, settings: Arc<SettingsService>) -> YtDlpProcessBackend {
    let context = Arc::new(ToolsContext::new(Arc::clone(&manager), settings));
    YtDlpProcessBackend::new(YtDlpRunner::new(Some(manager)), context)
}

fn minutes(seconds: f64) -> String {
    let total = seconds.round() as u64;
    format!("{}:{:02}", total / 60, total % 60)
}

fn print_analysis(analysis: &Analysis) {
    match analysis {
        Analysis::Video { info } => {
            let who = info
                .artist
                .as_deref()
                .or(info.channel.as_deref())
                .unwrap_or("?");
            println!("{} — {who}", info.title);
            println!("id: {}", info.id);
            if let Some(duration) = info.duration {
                println!("duração: {}", minutes(duration));
            }
            println!("faixa oficial: {}", info.is_official_track);
            if let Some(album) = &info.album {
                println!("álbum: {album}");
            }
        }
        Analysis::Collection { info } => {
            println!("{}", info.title.as_deref().unwrap_or("(sem título)"));
            println!("{} itens", info.entries.len());
            for (n, entry) in info.entries.iter().enumerate() {
                let duration = entry.duration.map(minutes).unwrap_or_else(|| "-".into());
                println!(
                    "{:>3}. {}  {}  {duration}",
                    n + 1,
                    entry.id,
                    entry.title.as_deref().unwrap_or("")
                );
            }
        }
    }
}

/// Uma linha de progresso a cada 10 % (por etapa).
struct ProgressPrinter {
    last: Mutex<(String, i64)>,
}

impl ProgressPrinter {
    fn new() -> Self {
        Self {
            last: Mutex::new((String::new(), -1)),
        }
    }

    fn print(&self, event: PipelineEvent) {
        let (stage, percent, detail) = match event {
            PipelineEvent::Download(ProgressUpdate {
                downloaded,
                total,
                speed,
                ..
            }) => {
                let percent = total
                    .filter(|t| *t > 0)
                    .map(|t| (downloaded * 100 / t) as i64)
                    .unwrap_or(-1);
                let rate = speed
                    .map(|s| format!(" {:.1} MiB/s", s / 1_048_576.0))
                    .unwrap_or_default();
                ("baixando", percent, rate)
            }
            PipelineEvent::Convert(percent) => ("convertendo", i64::from(percent), String::new()),
            PipelineEvent::Analyzing => ("analisando", -1, String::new()),
            PipelineEvent::Identifying => ("identificando", -1, String::new()),
            PipelineEvent::SourceSwitched { .. } | PipelineEvent::Identified(_) => return,
        };
        let bucket = if percent < 0 { -1 } else { percent / 10 };
        let mut last = self.last.lock().expect("progresso");
        if last.0 != stage || last.1 != bucket {
            *last = (stage.to_string(), bucket);
            if percent < 0 {
                eprintln!("[{stage}]{detail}");
            } else {
                eprintln!("[{stage}] {percent}%{detail}");
            }
        }
    }
}

pub async fn analyze(
    manager: Arc<ToolsManager>,
    settings: Arc<SettingsService>,
    url: &str,
    json: bool,
) -> Result<()> {
    if !matches!(
        classify(url),
        UrlKind::Video { .. } | UrlKind::Collection { .. }
    ) {
        bail!("URL não suportada: {url}");
    }
    let analysis = backend(manager, settings)
        .analyze(url, &cancel_on_ctrl_c())
        .await
        .map_err(failure)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&analysis)?);
    } else {
        print_analysis(&analysis);
    }
    Ok(())
}

pub async fn search(
    manager: Arc<ToolsManager>,
    settings: Arc<SettingsService>,
    text: &str,
    source: &str,
    limit: u32,
) -> Result<()> {
    let source = SearchSource::from_id(source)
        .with_context(|| format!("fonte desconhecida: {source} (use ytmusic ou youtube)"))?;
    let results = backend(manager, settings)
        .search(source, text, limit, &cancel_on_ctrl_c())
        .await
        .map_err(failure)?;
    for result in results {
        println!("{}\t{}", result.id, result.title);
    }
    Ok(())
}

pub async fn download(
    manager: Arc<ToolsManager>,
    settings: Arc<SettingsService>,
    data_dir: PathBuf,
    url: &str,
    profile_id: &str,
    out: &Path,
) -> Result<()> {
    let chosen = profile(profile_id).with_context(|| {
        let ids: Vec<_> = PROFILES.iter().map(|p| p.id).collect();
        format!("perfil desconhecido: {profile_id} (use {})", ids.join(", "))
    })?;
    let url = match classify(url) {
        UrlKind::Video { url, .. } => url,
        _ => bail!("o download aceita apenas a URL de um vídeo: {url}"),
    };
    let ffmpeg = manager
        .resolve(Tool::Ffmpeg)
        .context("FFmpeg não está instalado: rode `reverb-cli tools install`")?;
    let ffmpeg_dir = ffmpeg.parent().context("FFmpeg sem pasta")?.to_path_buf();

    sweep_orphans(&data_dir, std::time::Duration::from_secs(24 * 3600));
    let pipeline = DownloadPipeline::new(
        Arc::new(backend(Arc::clone(&manager), settings)),
        data_dir,
        ffmpeg_dir,
        Some(manager),
    );
    let job = PipelineJob {
        job_id: format!("cli-{}", std::process::id()),
        url,
        profile: chosen,
        out_dir: std::path::absolute(out)
            .with_context(|| format!("caminho inválido: {}", out.display()))?,
        sponsorblock: None,
        metadata_override: None,
        fetch_metadata: None,
    };
    let printer = ProgressPrinter::new();
    let output = pipeline
        .run(&job, &cancel_on_ctrl_c(), &|event| printer.print(event))
        .await
        .map_err(failure)?;
    println!("{}", output.path.display());
    Ok(())
}

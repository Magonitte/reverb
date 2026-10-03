//! Construtores **puros** dos argumentos do yt-dlp (arquitetura §8). Cada caminho e cada valor é
//! um único argumento (lista, nunca string de shell).

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CookiesArg {
    /// `--cookies-from-browser <navegador>`
    Browser(String),
    /// `--cookies <arquivo>`
    File(PathBuf),
}

/// Tudo o que as chamadas ao yt-dlp têm em comum.
#[derive(Debug, Clone)]
pub struct YtDlpContext {
    pub ytdlp_path: PathBuf,
    /// `deno:<caminho>` ou `node:<caminho>` (`JsRuntimeChoice::js_runtime_arg`).
    pub js_runtime_arg: String,
    pub ffmpeg_dir: PathBuf,
    pub cookies: Option<CookiesArg>,
    /// `--limit-rate <N>M` quando maior que zero.
    pub limit_rate_mbps: Option<f64>,
    /// Argumentos do provedor de PO token (`PotServer`/`ToolsManager::pot_args`).
    pub pot_args: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchSource {
    YtMusic,
    Youtube,
    Archive,
    Jamendo,
}

impl SearchSource {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "ytmusic" => Some(Self::YtMusic),
            "youtube" => Some(Self::Youtube),
            "archive" => Some(Self::Archive),
            "jamendo" => Some(Self::Jamendo),
            _ => None,
        }
    }
}

/// Opções de um job de download.
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub url: String,
    /// Pasta temporária do job (`JobWorkspace`).
    pub tmp_dir: PathBuf,
    /// Categorias do SponsorBlock a remover; `None` ou vazio = não usar.
    pub sponsorblock: Option<Vec<String>>,
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn rate(mbps: f64) -> String {
    format!("{mbps}M")
}

/// Argumentos fixos de toda chamada (§8).
pub fn base_args(ctx: &YtDlpContext) -> Vec<String> {
    let mut args = vec![
        "--ignore-config".to_string(),
        "--color".to_string(),
        "never".to_string(),
        "--js-runtimes".to_string(),
        ctx.js_runtime_arg.clone(),
        "--ffmpeg-location".to_string(),
        text(&ctx.ffmpeg_dir),
    ];
    match &ctx.cookies {
        Some(CookiesArg::Browser(browser)) => {
            args.extend(["--cookies-from-browser".to_string(), browser.clone()]);
        }
        Some(CookiesArg::File(file)) => args.extend(["--cookies".to_string(), text(file)]),
        None => {}
    }
    if let Some(mbps) = ctx.limit_rate_mbps.filter(|mbps| *mbps > 0.0) {
        args.extend(["--limit-rate".to_string(), rate(mbps)]);
    }
    if let Some(pot) = &ctx.pot_args {
        args.extend(pot.iter().cloned());
    }
    args
}

/// Analisar um vídeo: `-J --no-playlist -- <url>`.
pub fn analyze_video_args(ctx: &YtDlpContext, url: &str) -> Vec<String> {
    let mut args = base_args(ctx);
    args.extend(["-J", "--no-playlist", "--", url].map(String::from));
    args
}

/// Analisar uma coleção (playlist, álbum, canal): `-J --flat-playlist -- <url>`.
pub fn analyze_collection_args(ctx: &YtDlpContext, url: &str) -> Vec<String> {
    let mut args = base_args(ctx);
    args.extend(["-J", "--flat-playlist", "--", url].map(String::from));
    args
}

/// Busca por texto. No YouTube Music, a URL de busca com `#songs`; no YouTube, `ytsearch<n>:`.
pub fn search_args(
    ctx: &YtDlpContext,
    source: SearchSource,
    query: &str,
    limit: u32,
) -> Vec<String> {
    let mut args = base_args(ctx);
    match source {
        SearchSource::YtMusic => {
            let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
            args.extend([
                "-J".to_string(),
                "--flat-playlist".to_string(),
                "--playlist-end".to_string(),
                limit.to_string(),
                "--".to_string(),
                format!("https://music.youtube.com/search?q={encoded}#songs"),
            ]);
        }
        SearchSource::Youtube | SearchSource::Archive | SearchSource::Jamendo => {
            args.extend([
                "-J".to_string(),
                "--flat-playlist".to_string(),
                "--".to_string(),
                format!("ytsearch{limit}:{query}"),
            ]);
        }
    }
    args
}

/// Baixar o melhor áudio sem recodificar.
pub fn download_args(ctx: &YtDlpContext, options: &DownloadOptions) -> Vec<String> {
    let mut args = base_args(ctx);
    args.extend(
        [
            "--no-playlist",
            "--newline",
            "--progress",
            "-f",
            "bestaudio[format_id!*=-drc]/bestaudio",
            "-x",
            "--audio-format",
            "best",
            "--progress-template",
            "download:REVERB_PROGRESS %(progress)j",
            "--print",
            "after_move:REVERB_DONE %(.{id,title,filepath,ext,abr,acodec,format_id,duration})j",
        ]
        .map(String::from),
    );
    if crate::sources::provider_id(&options.url) == "soundcloud" {
        let index = args
            .iter()
            .position(|a| a == "-f")
            .expect("format argument")
            + 1;
        args[index] = "download/bestaudio".into();
    }
    if let Some(categories) = options.sponsorblock.as_ref().filter(|c| !c.is_empty()) {
        args.extend(["--sponsorblock-remove".to_string(), categories.join(",")]);
    }
    args.extend([
        "-o".to_string(),
        text(&options.tmp_dir.join("%(id)s.%(ext)s")),
        "--".to_string(),
        options.url.clone(),
    ]);
    args
}

#[cfg(test)]
mod tests;

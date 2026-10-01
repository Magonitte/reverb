//! Motor de download: argumentos, parsers, erros e runner do yt-dlp (F03, arquitetura §8/§9).

pub mod args;
pub mod errors;
pub mod models;
pub mod parse;
pub mod runner;

pub use args::{
    analyze_collection_args, analyze_video_args, download_args, search_args, CookiesArg,
    DownloadOptions, SearchSource, YtDlpContext,
};
pub use errors::{classify_stderr, DownloadError, ErrorKind};
pub use models::{
    AudioFormat, Chapter, CollectionEntry, CollectionInfo, SearchResult, Thumbnail, VideoInfo,
};
pub use parse::{parse_done_line, parse_progress_line, DoneInfo, ProgressUpdate};
pub use runner::{Analysis, RunnerConfig, YtDlpRunner};

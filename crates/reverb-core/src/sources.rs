//! Free public sources. Providers resolve their own URLs; signed download links are never logged.
use crate::backend::{DownloadBackend, JobSpec, ProgressCallback};
use crate::ytdlp::{
    Analysis, CollectionEntry, CollectionInfo, DoneInfo, DownloadError, ErrorKind, ProgressUpdate,
    SearchResult, SearchSource, VideoInfo,
};
use crate::{urlkind::UrlKind, SettingsService};
use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::json;
use std::{path::Path, sync::Arc, time::Duration};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;
use url::Url;

pub mod parsers;

pub fn provider_id(input: &str) -> &'static str {
    let Ok(url) = Url::parse(input) else {
        return "youtube";
    };
    match url.host_str().unwrap_or("") {
        "soundcloud.com" | "www.soundcloud.com" => "soundcloud",
        "archive.org" | "www.archive.org" => "archive",
        "jamendo.com" | "www.jamendo.com" => "jamendo",
        host if host.ends_with(".bandcamp.com") && host != "www.bandcamp.com" => "bandcamp",
        _ => "youtube",
    }
}

pub fn matches(input: &str) -> Option<UrlKind> {
    let mut url = Url::parse(input).ok()?;
    if !matches!(url.scheme(), "https" | "http")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
    let provider = provider_id(input);
    let (collection, id) = match (provider, parts.as_slice()) {
        ("archive", ["details", id]) if !id.is_empty() => (true, id.to_string()),
        ("archive", ["download", id, file]) if !id.is_empty() && !file.is_empty() => {
            (false, format!("{id}/{file}"))
        }
        ("soundcloud", [artist, "sets", set]) if !artist.is_empty() && !set.is_empty() => {
            (true, set.to_string())
        }
        ("soundcloud", [artist, track])
            if !artist.is_empty() && !track.is_empty() && *track != "sets" =>
        {
            (false, format!("{artist}/{track}"))
        }
        ("bandcamp", ["album", slug]) if !slug.is_empty() => (true, slug.to_string()),
        ("bandcamp", ["track", slug]) if !slug.is_empty() => (false, slug.to_string()),
        ("jamendo", ["track", id, ..]) if id.parse::<u64>().is_ok() => (false, id.to_string()),
        _ => return None,
    };
    url.set_fragment(None);
    // Archive selection is represented by a query on the public item URL.
    if provider != "archive" {
        url.set_query(None);
    }
    Some(
        if collection && !(provider == "archive" && url.query_pairs().any(|(k, _)| k == "file")) {
            UrlKind::Collection { url: url.into() }
        } else {
            UrlKind::Video {
                source_id: format!("{provider}:{id}"),
                url: url.into(),
                playlist_hint: false,
            }
        },
    )
}

#[derive(Clone, Debug)]
pub struct SourceTrack {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub year: Option<u32>,
    pub track_no: Option<u32>,
    pub url: String,
    pub download_url: String,
    pub ext: String,
    pub duration: Option<f64>,
}

impl SourceTrack {
    pub fn video(&self, provider: &str) -> Result<VideoInfo, DownloadError> {
        VideoInfo::from_json(&json!({"id":self.id,"title":self.title,"track":self.title,"artist":self.artist,"album":self.album,"release_year":self.year,"duration":self.duration,"webpage_url":self.url,"extractor_key":provider,"categories":[format!("reverb-track:{}",self.track_no.unwrap_or(0))],"formats":[{"format_id":"original","vcodec":"none","acodec":self.ext,"ext":self.ext}]}).to_string())
            .map_err(|_| error("Invalid source metadata"))
    }
}

fn error(message: &str) -> DownloadError {
    DownloadError::new(ErrorKind::Unavailable, message)
}
fn network() -> DownloadError {
    DownloadError::new(ErrorKind::Network, "Source request failed")
}
fn request_error(error: reqwest::Error) -> DownloadError {
    let error = error.without_url();
    let mut causes = Vec::new();
    let mut source = std::error::Error::source(&error);
    while let Some(e) = source {
        causes.push(e.to_string());
        source = e.source();
    }
    DownloadError::new(
        ErrorKind::Network,
        format!("{error}: {}", causes.join(": ")),
    )
}

#[async_trait]
pub trait SourceProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn matches(&self, url: &str) -> Option<UrlKind>;
    async fn analyze(
        &self,
        url: &str,
        cancel: &CancellationToken,
    ) -> Result<Analysis, DownloadError>;
    async fn search(
        &self,
        query: &str,
        limit: u32,
        cancel: &CancellationToken,
    ) -> Result<Vec<SearchResult>, DownloadError>;
    async fn download(
        &self,
        job: &JobSpec,
        progress: ProgressCallback<'_>,
        cancel: &CancellationToken,
    ) -> Result<DoneInfo, DownloadError>;
}

pub struct YtDlpSource {
    pub backend: Arc<dyn DownloadBackend>,
    pub provider: &'static str,
}
#[async_trait]
impl SourceProvider for YtDlpSource {
    fn id(&self) -> &'static str {
        self.provider
    }
    fn matches(&self, url: &str) -> Option<UrlKind> {
        if self.provider == "youtube" {
            let kind = crate::urlkind::classify_youtube_input(url);
            (!matches!(kind, UrlKind::Unsupported | UrlKind::Search { .. })).then_some(kind)
        } else {
            matches(url).filter(|_| provider_id(url) == self.provider)
        }
    }
    async fn analyze(
        &self,
        url: &str,
        cancel: &CancellationToken,
    ) -> Result<Analysis, DownloadError> {
        self.backend.analyze(url, cancel).await
    }
    async fn search(
        &self,
        q: &str,
        limit: u32,
        cancel: &CancellationToken,
    ) -> Result<Vec<SearchResult>, DownloadError> {
        self.backend
            .search(SearchSource::Youtube, q, limit, cancel)
            .await
    }
    async fn download(
        &self,
        job: &JobSpec,
        progress: ProgressCallback<'_>,
        cancel: &CancellationToken,
    ) -> Result<DoneInfo, DownloadError> {
        self.backend.download(job, progress, cancel).await
    }
}

pub struct HttpSource {
    pub provider: &'static str,
    pub client: reqwest::Client,
    pub settings: Arc<SettingsService>,
    pub archive_base: String,
    pub jamendo_base: String,
}
impl HttpSource {
    pub fn new(provider: &'static str, settings: Arc<SettingsService>) -> Self {
        Self {
            provider,
            settings,
            client: reqwest::Client::builder()
                .http1_only()
                .user_agent(crate::metadata::provider::user_agent())
                .connect_timeout(Duration::from_secs(20))
                .timeout(Duration::from_secs(600))
                .build()
                .expect("source HTTP client"),
            archive_base: "https://archive.org".into(),
            jamendo_base: "https://api.jamendo.com/v3.0".into(),
        }
    }
    async fn get(&self, url: &str, cancel: &CancellationToken) -> Result<String, DownloadError> {
        tokio::select! { _=cancel.cancelled()=>Err(DownloadError::cancelled()), result=async {
            self.client.get(url).send().await.map_err(request_error)?.error_for_status().map_err(request_error)?.text().await.map_err(request_error)
        }=>result }
    }
    pub async fn tracks(
        &self,
        input: &str,
        cancel: &CancellationToken,
    ) -> Result<Vec<SourceTrack>, DownloadError> {
        let url = Url::parse(input).map_err(|_| error("Invalid source URL"))?;
        match self.provider {
            "archive" => {
                let id = url
                    .path_segments()
                    .and_then(|mut p| p.nth(1))
                    .ok_or_else(|| error("Invalid Archive item"))?;
                let text = self
                    .get(&format!("{}/metadata/{id}", self.archive_base), cancel)
                    .await?;
                let mut tracks = parsers::archive(&text, id, &self.archive_base)?;
                let selected = url
                    .query_pairs()
                    .find(|(k, _)| k == "file")
                    .map(|(_, v)| v.into_owned())
                    .or_else(|| {
                        if url.path().starts_with("/download/") {
                            url.path_segments()
                                .and_then(|mut p| p.nth(2))
                                .and_then(|s| Url::parse(&format!("https://x/?f={s}")).ok())
                                .and_then(|u| u.query_pairs().next().map(|(_, v)| v.into_owned()))
                        } else {
                            None
                        }
                    });
                if let Some(file) = selected {
                    tracks.retain(|t| t.id == format!("{id}/{file}"));
                }
                Ok(tracks)
            }
            "jamendo" => {
                let id = url
                    .path_segments()
                    .and_then(|mut p| p.nth(1))
                    .ok_or_else(|| error("Invalid Jamendo track"))?;
                let key = self.settings.get().jamendo_client_id;
                if key.is_empty() {
                    return Err(error("Jamendo client ID required"));
                }
                let mut endpoint = Url::parse(&format!("{}/tracks/", self.jamendo_base))
                    .map_err(|_| error("Invalid endpoint"))?;
                endpoint.query_pairs_mut().extend_pairs([
                    ("client_id", key.as_str()),
                    ("format", "json"),
                    ("id", id),
                    ("audiodlformat", "flac"),
                ]);
                parsers::jamendo(&self.get(endpoint.as_str(), cancel).await?)
            }
            "bandcamp" => {
                let html = self.get(input, cancel).await?;
                let album = parsers::attribute_json(&html, "data-tralbum")?;
                let page = parsers::bandcamp_free_page(&album)?;
                let blob = parsers::attribute_json(&self.get(&page, cancel).await?, "data-blob")?;
                parsers::bandcamp(&album, &blob, input)
            }
            _ => Err(error("Unknown source")),
        }
    }

    async fn bandcamp_ready(
        &self,
        url: &str,
        cancel: &CancellationToken,
    ) -> Result<String, DownloadError> {
        let mut next = url.to_string();
        for _ in 0..8 {
            let mut status =
                Url::parse(&next).map_err(|_| error("Invalid Bandcamp download link"))?;
            if status.scheme() != "https"
                || !status
                    .host_str()
                    .is_some_and(|h| h.ends_with(".bandcamp.com"))
            {
                return Err(error("Invalid Bandcamp download host"));
            }
            status.set_path(&status.path().replace("/download/", "/statdownload/"));
            status
                .query_pairs_mut()
                .append_pair(".rand", &crate::queue::repo::now().to_string());
            let body = self.get(status.as_str(), cancel).await?;
            let value = parsers::bandcamp_status(&body)?;
            if let Some(url) = value["download_url"].as_str() {
                return Ok(url.to_string());
            }
            if let Some(url) = value["retry_url"].as_str() {
                next = url.to_string();
            } else if value["result"].as_str() == Some("ok") {
                return Ok(next);
            } else if value["result"].as_str() != Some("pending") {
                return Err(error("Bandcamp download could not be prepared"));
            }
            tokio::select! {_=cancel.cancelled()=>return Err(DownloadError::cancelled()),_=tokio::time::sleep(Duration::from_secs(1))=>{}}
        }
        Err(network())
    }
}

#[async_trait]
impl SourceProvider for HttpSource {
    fn id(&self) -> &'static str {
        self.provider
    }
    fn matches(&self, url: &str) -> Option<UrlKind> {
        matches(url).filter(|_| provider_id(url) == self.provider)
    }
    async fn analyze(
        &self,
        url: &str,
        cancel: &CancellationToken,
    ) -> Result<Analysis, DownloadError> {
        let tracks = self.tracks(url, cancel).await?;
        if tracks.len() == 1 {
            return Ok(Analysis::Video {
                info: Box::new(tracks[0].video(self.provider)?),
            });
        }
        if tracks.is_empty() {
            return Err(error("No free downloadable audio"));
        }
        Ok(Analysis::Collection {
            info: CollectionInfo {
                id: Some(url.into()),
                title: tracks[0].album.clone(),
                channel: tracks[0].artist.clone(),
                thumbnail: None,
                entries: tracks
                    .into_iter()
                    .map(|t| CollectionEntry {
                        id: t.id,
                        title: Some(t.title),
                        duration: t.duration,
                        url: Some(t.url),
                    })
                    .collect(),
            },
        })
    }
    async fn search(
        &self,
        q: &str,
        limit: u32,
        cancel: &CancellationToken,
    ) -> Result<Vec<SearchResult>, DownloadError> {
        let mut endpoint = Url::parse(&format!(
            "{}/{}",
            if self.provider == "archive" {
                &self.archive_base
            } else {
                &self.jamendo_base
            },
            if self.provider == "archive" {
                "advancedsearch.php"
            } else {
                "tracks/"
            }
        ))
        .map_err(|_| error("Invalid endpoint"))?;
        if self.provider == "archive" {
            endpoint.query_pairs_mut().extend_pairs([
                ("q", format!("({q}) AND mediatype:audio").as_str()),
                ("fl[]", "identifier"),
                ("fl[]", "title"),
                ("fl[]", "creator"),
                ("rows", limit.to_string().as_str()),
                ("output", "json"),
            ]);
            parsers::archive_search(
                &self.get(endpoint.as_str(), cancel).await?,
                &self.archive_base,
            )
        } else if self.provider == "jamendo" {
            let key = self.settings.get().jamendo_client_id;
            if key.is_empty() {
                return Err(error("Jamendo client ID required"));
            }
            endpoint.query_pairs_mut().extend_pairs([
                ("client_id", key.as_str()),
                ("format", "json"),
                ("limit", limit.to_string().as_str()),
                ("search", q),
                ("audiodlformat", "flac"),
            ]);
            Ok(
                parsers::jamendo(&self.get(endpoint.as_str(), cancel).await?)?
                    .into_iter()
                    .map(|t| SearchResult {
                        id: t.id,
                        title: t.title,
                        url: Some(t.url),
                        duration: t.duration,
                        channel: t.artist,
                    })
                    .collect(),
            )
        } else {
            Err(error("Search unsupported"))
        }
    }
    async fn download(
        &self,
        job: &JobSpec,
        progress: ProgressCallback<'_>,
        cancel: &CancellationToken,
    ) -> Result<DoneInfo, DownloadError> {
        let tracks = self.tracks(&job.url, cancel).await?;
        if tracks.len() != 1 {
            return Err(error("Select one track from the collection"));
        }
        let track = &tracks[0];
        let path = job.tmp_dir.join(format!("source.{}", track.ext));
        let download_url = if self.provider == "bandcamp" {
            self.bandcamp_ready(&track.download_url, cancel).await?
        } else {
            track.download_url.clone()
        };
        download_http(&self.client, &download_url, &path, progress, cancel).await?;
        Ok(DoneInfo {
            id: track.id.clone(),
            title: track.title.clone(),
            filepath: path.to_string_lossy().into_owned(),
            ext: track.ext.clone(),
            abr: None,
            acodec: Some(track.ext.clone()),
            format_id: Some("original".into()),
            duration: track.duration,
        })
    }
}

pub async fn download_http(
    client: &reqwest::Client,
    url: &str,
    path: &Path,
    progress: ProgressCallback<'_>,
    cancel: &CancellationToken,
) -> Result<(), DownloadError> {
    let partial = path.with_extension("part");
    let result = tokio::select! { _=cancel.cancelled()=>Err(DownloadError::cancelled()), result=async {
        let response=client.get(url).send().await.map_err(request_error)?.error_for_status().map_err(request_error)?;
        let total=response.content_length();
        let mut stream=response.bytes_stream();
        let mut file=tokio::fs::File::create(&partial).await.map_err(|_|DownloadError::new(ErrorKind::Disk,"Cannot create download"))?;
        let mut downloaded=0;
        let started=std::time::Instant::now();
        while let Some(chunk)=stream.next().await {
            let chunk=chunk.map_err(|_|network())?;
            file.write_all(&chunk).await.map_err(|_|DownloadError::new(ErrorKind::Disk,"Cannot write download"))?;
            downloaded+=chunk.len() as u64;
            progress(ProgressUpdate{downloaded,total,speed:Some(downloaded as f64/started.elapsed().as_secs_f64().max(0.001)),eta:None,finished:false});
        }
        if downloaded==0 || total.is_some_and(|n|n!=downloaded){return Err(network());}
        file.flush().await.map_err(|_|DownloadError::new(ErrorKind::Disk,"Cannot flush download"))?;
        drop(file);
        tokio::fs::rename(&partial,path).await.map_err(|_|DownloadError::new(ErrorKind::Disk,"Cannot publish download"))?;
        progress(ProgressUpdate{downloaded,total,speed:None,eta:Some(0),finished:true});
        Ok(())
    }=>result };
    if result.is_err() {
        let _ = tokio::fs::remove_file(&partial).await;
    }
    result
}

pub struct SourcesBackend {
    youtube: Arc<dyn DownloadBackend>,
    providers: Vec<Arc<dyn SourceProvider>>,
    soundcloud_gate: tokio::sync::Mutex<Option<tokio::time::Instant>>,
}
impl SourcesBackend {
    pub fn new(youtube: Arc<dyn DownloadBackend>, settings: Arc<SettingsService>) -> Self {
        let mut providers: Vec<Arc<dyn SourceProvider>> = vec![
            Arc::new(YtDlpSource {
                backend: youtube.clone(),
                provider: "youtube",
            }),
            Arc::new(YtDlpSource {
                backend: youtube.clone(),
                provider: "soundcloud",
            }),
        ];
        for provider in ["archive", "bandcamp", "jamendo"] {
            providers.push(Arc::new(HttpSource::new(provider, settings.clone())));
        }
        Self {
            youtube,
            providers,
            soundcloud_gate: tokio::sync::Mutex::new(None),
        }
    }
    fn source(&self, url: &str) -> Result<&Arc<dyn SourceProvider>, DownloadError> {
        self.providers
            .iter()
            .find(|p| p.matches(url).is_some())
            .ok_or_else(|| error("Unsupported source URL"))
    }
}
#[async_trait]
impl DownloadBackend for SourcesBackend {
    async fn analyze(
        &self,
        url: &str,
        cancel: &CancellationToken,
    ) -> Result<Analysis, DownloadError> {
        self.source(url)?.analyze(url, cancel).await
    }
    async fn search(
        &self,
        source: SearchSource,
        q: &str,
        limit: u32,
        cancel: &CancellationToken,
    ) -> Result<Vec<SearchResult>, DownloadError> {
        match source {
            SearchSource::Archive | SearchSource::Jamendo => {
                self.providers
                    .iter()
                    .find(|p| {
                        p.id()
                            == if source == SearchSource::Archive {
                                "archive"
                            } else {
                                "jamendo"
                            }
                    })
                    .ok_or_else(|| error("Unknown source"))?
                    .search(q, limit, cancel)
                    .await
            }
            _ => self.youtube.search(source, q, limit, cancel).await,
        }
    }
    async fn download(
        &self,
        job: &JobSpec,
        progress: ProgressCallback<'_>,
        cancel: &CancellationToken,
    ) -> Result<DoneInfo, DownloadError> {
        let source = self.source(&job.url)?;
        if source.id() == "soundcloud" {
            let mut gate = self.soundcloud_gate.lock().await;
            if let Some(last) = *gate {
                tokio::select! {_=cancel.cancelled()=>return Err(DownloadError::cancelled()),_=tokio::time::sleep_until(last+Duration::from_secs(10))=>{}}
            }
            let result = source.download(job, progress, cancel).await;
            *gate = Some(tokio::time::Instant::now());
            result
        } else {
            source.download(job, progress, cancel).await
        }
    }
}

#[cfg(test)]
mod tests;

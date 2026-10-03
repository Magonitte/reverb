use super::*;
use crate::backend::DownloadBackend;
use crate::metadata::official::{find_imported, OfficialVia};
use crate::queue::{EnqueueRequest, Job, JobOptions, PlaylistCtx, QueueService};
use crate::{CoreError, CoreResult, Db, EventSink, SettingsService};
use futures_util::{stream, StreamExt};
use std::{collections::HashMap, sync::Arc};
use tokio_util::sync::CancellationToken;

// Candidate analyses inside one match share the same limit across all tracks.
struct BoundedBackend {
    inner: Arc<dyn DownloadBackend>,
    slots: tokio::sync::Semaphore,
}
impl BoundedBackend {
    async fn permit(
        &self,
        cancel: &CancellationToken,
    ) -> Result<tokio::sync::SemaphorePermit<'_>, crate::ytdlp::errors::DownloadError> {
        let cancelled = || {
            crate::ytdlp::errors::DownloadError::new(
                crate::ytdlp::errors::ErrorKind::Cancelled,
                "Import cancelled",
            )
        };
        tokio::select! {
            _ = cancel.cancelled() => Err(cancelled()),
            slot = self.slots.acquire() => slot.map_err(|_| cancelled()),
        }
    }
}
#[async_trait::async_trait]
impl DownloadBackend for BoundedBackend {
    async fn analyze(
        &self,
        url: &str,
        cancel: &CancellationToken,
    ) -> Result<crate::ytdlp::Analysis, crate::ytdlp::errors::DownloadError> {
        let _slot = self.permit(cancel).await?;
        self.inner.analyze(url, cancel).await
    }
    async fn search(
        &self,
        source: crate::ytdlp::SearchSource,
        query: &str,
        limit: u32,
        cancel: &CancellationToken,
    ) -> Result<Vec<crate::ytdlp::SearchResult>, crate::ytdlp::errors::DownloadError> {
        let _slot = self.permit(cancel).await?;
        self.inner.search(source, query, limit, cancel).await
    }
    async fn download(
        &self,
        job: &crate::backend::JobSpec,
        progress: crate::backend::ProgressCallback<'_>,
        cancel: &CancellationToken,
    ) -> Result<crate::ytdlp::DoneInfo, crate::ytdlp::errors::DownloadError> {
        self.inner.download(job, progress, cancel).await
    }
}
#[cfg(test)]
mod tests;

pub struct ImportService {
    pub deezer: Arc<DeezerSource>,
    spotify: SpotifySource,
    backend: Arc<dyn DownloadBackend>,
    pub db: Db,
    pub settings: Arc<SettingsService>,
    pub queue: QueueService,
    sink: Arc<dyn EventSink>,
    analyses: tokio::sync::Mutex<HashMap<String, ImportAnalysis>>,
}
impl ImportService {
    pub fn new(
        db: Db,
        settings: Arc<SettingsService>,
        backend: Arc<dyn DownloadBackend>,
        queue: QueueService,
        sink: Arc<dyn EventSink>,
    ) -> Arc<Self> {
        Self::with_sources(
            db,
            settings.clone(),
            backend,
            queue,
            sink,
            Arc::new(DeezerSource::default()),
            SpotifySource::new(settings),
        )
    }
    pub fn with_sources(
        db: Db,
        settings: Arc<SettingsService>,
        backend: Arc<dyn DownloadBackend>,
        queue: QueueService,
        sink: Arc<dyn EventSink>,
        deezer: Arc<DeezerSource>,
        spotify: SpotifySource,
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            settings,
            backend: Arc::new(BoundedBackend {
                inner: backend,
                slots: tokio::sync::Semaphore::new(3),
            }),
            queue,
            sink,
            deezer,
            spotify,
            analyses: tokio::sync::Mutex::new(HashMap::new()),
        })
    }
    pub async fn fetch(&self, url: &str) -> CoreResult<ImportedCollection> {
        if self.settings.get().offline_mode {
            return Err(CoreError::coded(
                "offline",
                "Import requires network access",
            ));
        }
        if self.deezer.matches(url) {
            self.deezer.fetch(url).await
        } else if self.spotify.matches(url) {
            self.spotify.fetch(url).await
        } else {
            Err(CoreError::invalid("Unsupported import URL"))
        }
    }
    pub async fn match_track(&self, track: &ImportedTrack) -> CoreResult<MatchResult> {
        let subject = crate::metadata::score::Subject {
            title: track.fields.title.clone(),
            artists: track.fields.artists.clone(),
            duration_s: track.duration_s,
        };
        let found = find_imported(
            self.backend.as_ref(),
            &subject,
            track.isrc.as_deref(),
            &CancellationToken::new(),
        )
        .await;
        Ok(match found {
            Some(found) => MatchResult {
                video_id: Some(found.matched.video_id),
                confidence: found.matched.score,
                via: found.matched.via,
                bucket: if found.matched.score >= self.settings.get().confidence_auto_apply {
                    "ok"
                } else {
                    "review"
                }
                .into(),
            },
            None => MatchResult {
                video_id: None,
                confidence: 0.,
                via: OfficialVia::Text,
                bucket: "none".into(),
            },
        })
    }
    pub async fn analyze(&self, url: &str) -> CoreResult<ImportAnalysis> {
        let collection = self.fetch(url).await?;
        let total = collection.tracks.len();
        let mut processed = 0;
        let mut tasks = stream::iter(collection.tracks.iter().cloned().enumerate().map(
            |(index, track)| async move {
                let matched = self.match_track(&track).await?;
                CoreResult::Ok((index, ImportedItem { track, matched }))
            },
        ))
        .buffer_unordered(3);
        let mut ordered = Vec::new();
        while let Some(item) = tasks.next().await {
            ordered.push(item?);
            processed += 1;
            self.sink.emit(
                "import://progress",
                serde_json::json!({"processed":processed,"total":total,"url":url}),
            );
        }
        ordered.sort_by_key(|(i, _)| *i);
        let analysis = ImportAnalysis {
            collection: collection.clone(),
            items: ordered.into_iter().map(|(_, i)| i).collect(),
        };
        let mut cache = self.analyses.lock().await;
        if cache.len() >= 8 {
            cache.clear();
        }
        cache.insert(url.into(), analysis.clone());
        cache.insert(collection.url.clone(), analysis.clone());
        Ok(analysis)
    }
    pub async fn enqueue_item(
        &self,
        item: &ImportedItem,
        profile: &str,
        output: Option<String>,
        ctx: Option<PlaylistCtx>,
    ) -> CoreResult<Option<Job>> {
        let Some(id) = &item.matched.video_id else {
            return Ok(None);
        };
        let mut meta = serde_json::to_value(&item.track.fields)?;
        meta["isrc"] = serde_json::to_value(&item.track.isrc)?;
        meta["importMatch"] = serde_json::to_value(&item.matched)?;
        meta["importSourceTrackId"] = serde_json::to_value(&item.track.id)?;
        let request = EnqueueRequest {
            url: format!("https://music.youtube.com/watch?v={id}"),
            source_id: Some(id.clone()),
            title: Some(item.track.fields.title.clone()),
            thumbnail: item.track.fields.cover_url.clone(),
            duration_s: item.track.duration_s,
            profile_id: Some(profile.into()),
            metadata_override: Some(meta),
            playlist_ctx: ctx,
            options: Some(JobOptions {
                output_dir: output,
                ..Default::default()
            }),
            ..Default::default()
        };
        match self.queue.enqueue(request).await {
            Ok(job) => Ok(Some(job)),
            Err(e) if e.kind() == "duplicate" => Ok(None),
            Err(e) => Err(e),
        }
    }
    pub async fn enqueue(
        &self,
        selection: ImportSelection,
        syncs: &crate::sync::SyncService,
    ) -> CoreResult<Vec<Job>> {
        if !matches!(selection.mode.as_str(), "once" | "sync") {
            return Err(CoreError::invalid("Invalid import mode"));
        }
        let cached = self
            .analyses
            .lock()
            .await
            .get(&selection.url)
            .cloned()
            .ok_or_else(|| CoreError::coded("import_expired", "Analyze the collection again"))?;
        if selection.mode == "sync" {
            let mut options = selection
                .sync_options
                .ok_or_else(|| CoreError::invalid("Sync options required"))?;
            options.url = selection.url;
            let sync = syncs.create(options).await?;
            syncs.run(&sync.id).await?;
            return Ok(Vec::new());
        }
        let mut jobs = Vec::new();
        for (i, item) in cached.items.iter().enumerate().filter(|(_, i)| {
            selection.track_ids.contains(&i.track.id) && i.matched.bucket != "none"
        }) {
            if let Some(job) = self
                .enqueue_item(
                    item,
                    &selection.profile_id,
                    None,
                    Some(PlaylistCtx {
                        playlist_title: cached.collection.title.clone(),
                        playlist_id: cached.collection.id.clone(),
                        index: i as u32 + 1,
                        sync_id: None,
                    }),
                )
                .await?
            {
                jobs.push(job);
            }
        }
        Ok(jobs)
    }
}

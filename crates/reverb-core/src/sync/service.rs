use super::{files, plan_sync, repo, Sync, SyncCreate, SyncItem, SyncResult, SyncUpdate};
use crate::backend::DownloadBackend;
use crate::queue::{EnqueueRequest, JobOptions, PlaylistCtx, QueueService};
use crate::ytdlp::{Analysis, CollectionInfo};
use crate::{CoreError, CoreResult, Db, EventSink, SettingsService};
use rusqlite::params;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub struct SyncService {
    db: Db,
    settings: Arc<SettingsService>,
    backend: Arc<dyn DownloadBackend>,
    queue: QueueService,
    sink: Arc<dyn EventSink>,
    ops: tokio::sync::Mutex<()>,
    clock: Arc<dyn Fn() -> i64 + Send + std::marker::Sync>,
    stop: CancellationToken,
}

pub fn is_due(sync: &Sync, now: i64) -> bool {
    sync.enabled
        && sync.interval_hours > 0
        && !sync
            .last_result
            .as_ref()
            .is_some_and(|result| result.running)
        && sync
            .last_sync_at
            .is_none_or(|last| now.saturating_sub(last) >= i64::from(sync.interval_hours) * 3600)
}

impl SyncService {
    pub fn new(
        db: Db,
        settings: Arc<SettingsService>,
        backend: Arc<dyn DownloadBackend>,
        queue: QueueService,
        sink: Arc<dyn EventSink>,
    ) -> Arc<Self> {
        Self::with_clock(
            db,
            settings,
            backend,
            queue,
            sink,
            Arc::new(crate::queue::repo::now),
        )
    }
    pub fn with_clock(
        db: Db,
        settings: Arc<SettingsService>,
        backend: Arc<dyn DownloadBackend>,
        queue: QueueService,
        sink: Arc<dyn EventSink>,
        clock: Arc<dyn Fn() -> i64 + Send + std::marker::Sync>,
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            settings,
            backend,
            queue,
            sink,
            ops: tokio::sync::Mutex::new(()),
            clock,
            stop: CancellationToken::new(),
        })
    }
    fn changed(&self, id: &str) {
        self.sink
            .emit("sync://updated", serde_json::json!({"id":id}));
    }
    pub async fn list(&self) -> CoreResult<Vec<Sync>> {
        self.db.call(|conn| repo::list(conn)).await
    }
    pub async fn items(&self, id: &str) -> CoreResult<Vec<SyncItem>> {
        let id = id.to_owned();
        self.db.call(move |conn| repo::items(conn, &id)).await
    }
    async fn get(&self, id: &str) -> CoreResult<Sync> {
        let id = id.to_owned();
        self.db.call(move |conn| repo::get(conn, &id)).await
    }
    async fn analyze(&self, url: &str) -> CoreResult<CollectionInfo> {
        if !matches!(
            crate::urlkind::classify(url),
            crate::urlkind::UrlKind::Collection { .. }
        ) {
            return Err(CoreError::invalid("A collection URL is required"));
        }
        match self
            .backend
            .analyze(url, &self.stop.child_token())
            .await
            .map_err(|error| CoreError::coded(error.kind.as_str(), error.message))?
        {
            Analysis::Collection { info } => Ok(info),
            _ => Err(CoreError::invalid("The URL does not identify a collection")),
        }
    }
    pub async fn create(&self, request: SyncCreate) -> CoreResult<Sync> {
        validate(
            &request.profile_id,
            request.interval_hours,
            request.max_items,
        )?;
        let _ops = self.ops.lock().await;
        let info = self.analyze(&request.url).await?;
        let custom = request
            .title
            .as_deref()
            .is_some_and(|title| !title.trim().is_empty());
        let title = request
            .title
            .filter(|title| !title.trim().is_empty())
            .or(info.title)
            .or(info.channel)
            .unwrap_or_else(|| request.url.clone());
        let id = uuid::Uuid::new_v4().to_string();
        let now = (self.clock)();
        let saved = id.clone();
        let sync=self.db.call(move|conn| {
            let tx=conn.transaction()?;
            tx.execute("INSERT INTO syncs(id,url,playlist_id,title,thumbnail,profile_id,output_dir,interval_hours,max_items,remove_deleted,write_m3u,created_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",params![saved,request.url.trim(),info.id,title.trim(),info.thumbnail,request.profile_id,request.output_dir.filter(|path|!path.trim().is_empty()),request.interval_hours,request.max_items,request.remove_deleted,request.write_m3u,now])?;
            if custom {tx.execute("INSERT INTO kv(key,value) VALUES(?,?)",params![format!("sync.custom-title.{saved}"),"true"])?;}
            let sync=repo::get(&tx,&saved)?;tx.commit()?;Ok(sync)
        }).await?;
        self.changed(&id);
        Ok(sync)
    }
    pub async fn update(&self, id: &str, request: SyncUpdate) -> CoreResult<Sync> {
        validate(
            &request.profile_id,
            request.interval_hours,
            request.max_items,
        )?;
        if request.title.trim().is_empty() {
            return Err(CoreError::invalid("Playlist title is required"));
        }
        let _ops = self.ops.lock().await;
        let old = self.get(id).await?;
        if old
            .last_result
            .as_ref()
            .is_some_and(|result| result.running)
        {
            return Err(CoreError::coded("busy", "Playlist is synchronizing"));
        }
        let saved = id.to_owned();
        let sync=self.db.call(move|conn| {
            let tx=conn.transaction()?;
            repo::ensure_changed(tx.execute("UPDATE syncs SET title=?,profile_id=?,output_dir=?,interval_hours=?,max_items=?,remove_deleted=?,write_m3u=?,enabled=? WHERE id=?",params![request.title.trim(),request.profile_id,request.output_dir.filter(|path|!path.trim().is_empty()),request.interval_hours,request.max_items,request.remove_deleted,request.write_m3u,request.enabled,saved])?)?;
            if old.title!=request.title.trim() {tx.execute("INSERT OR REPLACE INTO kv(key,value) VALUES(?,?)",params![format!("sync.custom-title.{saved}"),"true"])?;}
            let sync=repo::get(&tx,&saved)?;tx.commit()?;Ok(sync)
        }).await?;
        self.refresh_names(&sync).await?;
        if sync.write_m3u {
            self.export(&sync).await?;
        }
        self.changed(id);
        Ok(sync)
    }
    pub async fn delete(&self, id: &str, delete_files: bool) -> CoreResult<()> {
        let _ops = self.ops.lock().await;
        let _sync = self.get(id).await?;
        for item in self.items(id).await? {
            if let Some(job) = item.job_id {
                if matches!(item.job_status.as_deref(), Some("queued" | "running")) {
                    self.queue.cancel(&job).await?;
                }
            }
        }
        let saved = id.to_owned();
        self.db
            .call(move |conn| {
                if delete_files {
                    let ids = repo::items(conn, &saved)?
                        .iter()
                        .filter_map(|item| item.library_id)
                        .collect::<Vec<_>>();
                    files::delete_tracks(conn, &ids)?;
                }
                conn.execute(
                    "DELETE FROM kv WHERE key=?",
                    [format!("sync.custom-title.{saved}")],
                )?;
                repo::ensure_changed(conn.execute("DELETE FROM syncs WHERE id=?", [saved])?)
            })
            .await?;
        self.sink.emit("library://changed", serde_json::json!({}));
        self.changed(id);
        Ok(())
    }
    pub async fn run(&self, id: &str) -> CoreResult<SyncResult> {
        let _ops = self.ops.lock().await;
        let mut sync = self.get(id).await?;
        if sync
            .last_result
            .as_ref()
            .is_some_and(|result| result.running)
        {
            return Err(CoreError::coded("busy", "Playlist is synchronizing"));
        }
        let info = match self.analyze(&sync.url).await {
            Ok(info) => info,
            Err(error) => {
                self.save_result(
                    id,
                    SyncResult {
                        error: Some(error.to_string()),
                        ..SyncResult::default()
                    },
                )
                .await?;
                return Err(error);
            }
        };
        let saved = id.to_owned();
        let remote_title = info.title.clone();
        let remote_id = info.id.clone();
        let thumbnail = info.thumbnail.clone();
        sync=self.db.call(move|conn| {
            let custom:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM kv WHERE key=?)",[format!("sync.custom-title.{saved}")],|row|row.get(0))?;
            if !custom {if let Some(title)=remote_title.filter(|title|!title.trim().is_empty()) {conn.execute("UPDATE syncs SET title=? WHERE id=?",params![title,saved])?;}}
            conn.execute("UPDATE syncs SET playlist_id=COALESCE(?,playlist_id),thumbnail=COALESCE(?,thumbnail) WHERE id=?",params![remote_id,thumbnail,saved])?;
            repo::get(conn,&saved)
        }).await?;
        let current = self.items(id).await?;
        let plan = plan_sync(&current, &info.entries, sync.max_items);
        let mut result = SyncResult {
            added: plan.add.len() as u32,
            removed: plan.removed.len() as u32,
            duplicates: plan.duplicates,
            unavailable: plan.unavailable,
            running: true,
            ..SyncResult::default()
        };
        self.save_result(id, result.clone()).await?;
        for source in plan.removed {
            if let Some(item) = current.iter().find(|item| item.source_id == source) {
                if let Some(job) = &item.job_id {
                    if matches!(item.job_status.as_deref(), Some("queued" | "running")) {
                        self.queue.cancel(job).await?;
                    }
                }
                let library = item.library_id;
                let remove = sync.remove_deleted;
                let saved = id.to_owned();
                let now = (self.clock)();
                self.db.call(move|conn| {
                    if remove {if let Some(library)=library {files::delete_tracks(conn,&[library])?;}}
                    conn.execute("UPDATE sync_items SET state='removed',removed_at=?,library_id=CASE WHEN ? THEN NULL ELSE library_id END WHERE sync_id=? AND source_id=?",params![now,remove,saved,source])?;Ok(())
                }).await?;
            }
        }
        for item in plan
            .add
            .into_iter()
            .chain(plan.reordered)
            .chain(plan.unchanged)
        {
            let previous = current.iter().find(|old| old.source_id == item.entry.id);
            let provider = sync.provider.clone();
            let source = item.entry.id.clone();
            let profile = sync.profile_id.clone();
            let previous_library = previous.and_then(|old| old.library_id);
            let library = self
                .db
                .call(move |conn| {
                    if let Some(id) = previous_library {
                        if let Some(track) = crate::library::get(conn, id)? {
                            if track.profile_id.as_deref() == Some(&profile)
                                && !track.missing
                                && std::path::Path::new(&track.file_path).is_file()
                            {
                                return Ok(Some(id));
                            }
                        }
                    }
                    repo::library_match(conn, &provider, &source, &profile)
                })
                .await?;
            let mut job = previous
                .filter(|old| {
                    old.state == "present"
                        && matches!(old.job_status.as_deref(), Some("queued" | "running"))
                })
                .and_then(|old| old.job_id.clone());
            if library.is_none() && job.is_none() {
                let request = EnqueueRequest {
                    url: format!("https://www.youtube.com/watch?v={}", item.entry.id),
                    source_id: Some(item.entry.id.clone()),
                    title: item.entry.title.clone(),
                    duration_s: item.entry.duration,
                    profile_id: Some(sync.profile_id.clone()),
                    playlist_ctx: Some(PlaylistCtx {
                        playlist_title: sync.title.clone(),
                        playlist_id: sync.playlist_id.clone().unwrap_or_else(|| sync.id.clone()),
                        index: item.position,
                        sync_id: Some(id.to_owned()),
                    }),
                    options: Some(JobOptions {
                        output_dir: sync.output_dir.clone(),
                        ..JobOptions::default()
                    }),
                    allow_duplicate: true,
                    ..EnqueueRequest::default()
                };
                match self.queue.enqueue(request).await {
                    Ok(queued) => job = Some(queued.id),
                    Err(error) => {
                        result.failed += 1;
                        result.error = Some(error.to_string());
                    }
                }
            }
            let saved = id.to_owned();
            let now = (self.clock)();
            self.db.call(move|conn| {
                conn.execute("INSERT INTO sync_items(sync_id,source_id,position,title,state,job_id,library_id,first_seen_at) VALUES(?,?,?,?,'present',?,?,?) ON CONFLICT(sync_id,source_id) DO UPDATE SET position=excluded.position,title=excluded.title,state='present',job_id=excluded.job_id,library_id=excluded.library_id,removed_at=NULL",params![saved,item.entry.id,item.position,item.entry.title,job,library,now])?;Ok(())
            }).await?;
        }
        self.refresh_names(&sync).await?;
        self.save_result(id, result.clone()).await?;
        self.sink.emit("library://changed", serde_json::json!({}));
        self.finalize_locked(id).await?;
        Ok(self.get(id).await?.last_result.unwrap_or(result))
    }
    async fn save_result(&self, id: &str, result: SyncResult) -> CoreResult<()> {
        let saved = id.to_owned();
        let now = (self.clock)();
        self.db
            .call(move |conn| repo::save_result(conn, &saved, &result, now))
            .await?;
        self.changed(id);
        Ok(())
    }
    async fn refresh_names(&self, sync: &Sync) -> CoreResult<()> {
        let settings = self.settings.get();
        if !(settings.file_template.contains("{playlist}")
            || settings.file_template.contains("{playlist_index"))
        {
            return Ok(());
        }
        for item in self
            .items(&sync.id)
            .await?
            .into_iter()
            .filter(|item| item.state == "present")
        {
            if let Some(library) = item.library_id {
                let sync = sync.clone();
                let settings = settings.clone();
                self.db
                    .call(move |conn| {
                        files::rename_item(conn, &sync, library, item.position, &settings)
                    })
                    .await?;
            }
        }
        Ok(())
    }
    async fn export(&self, sync: &Sync) -> CoreResult<()> {
        let sync = sync.clone();
        let settings = self.settings.get();
        self.db
            .call(move |conn| files::write_m3u(conn, &sync, &settings))
            .await?;
        Ok(())
    }
    async fn finalize_locked(&self, id: &str) -> CoreResult<()> {
        let sync = self.get(id).await?;
        let Some(mut result) = sync.last_result.clone().filter(|result| result.running) else {
            return Ok(());
        };
        let items = self.items(id).await?;
        if items.iter().any(|item| {
            item.state == "present"
                && matches!(item.job_status.as_deref(), Some("queued" | "running"))
        }) {
            return Ok(());
        }
        result.failed = items
            .iter()
            .filter(|item| item.state == "present" && (item.library_id.is_none() || item.missing))
            .count() as u32;
        result.running = false;
        if let Err(error) = self.refresh_names(&sync).await {
            result.error = Some(error.to_string());
        }
        if sync.write_m3u {
            if let Err(error) = self.export(&sync).await {
                result.error = Some(error.to_string());
            }
        }
        let saved = id.to_owned();
        let last = sync.last_sync_at.unwrap_or_else(|| (self.clock)());
        self.db
            .call(move |conn| repo::save_result(conn, &saved, &result, last))
            .await?;
        self.changed(id);
        Ok(())
    }
    pub async fn scheduled_tick(&self) -> CoreResult<()> {
        for sync in self
            .list()
            .await?
            .into_iter()
            .filter(|sync| is_due(sync, (self.clock)()))
        {
            if let Err(error) = self.run(&sync.id).await {
                tracing::warn!(%error,sync_id=sync.id,"Scheduled playlist synchronization failed");
            }
        }
        Ok(())
    }
    pub async fn reconcile(&self) -> CoreResult<()> {
        let _ops = self.ops.lock().await;
        for sync in self.list().await? {
            self.finalize_locked(&sync.id).await?;
        }
        Ok(())
    }
    pub fn start(self: &Arc<Self>, cancel: CancellationToken) {
        let service = Arc::clone(self);
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_millis(500));
            let mut next = tokio::time::Instant::now() + Duration::from_secs(120);
            loop {
                tokio::select! {
                    _=cancel.cancelled()=>{service.stop.cancel();break;},
                    _=tick.tick()=>{
                        let result=async {
                            service.reconcile().await?;
                            if tokio::time::Instant::now()>=next {next=tokio::time::Instant::now()+Duration::from_secs(900);service.scheduled_tick().await?;}
                            CoreResult::Ok(())
                        }.await;
                        if let Err(error)=result {tracing::warn!(%error,"Playlist maintenance failed");}
                    }
                }
            }
        });
    }
    pub fn stop(&self) {
        self.stop.cancel();
    }
}

fn validate(profile: &str, interval: u32, max: Option<u32>) -> CoreResult<()> {
    if crate::profiles::profile(profile).is_none() {
        return Err(CoreError::invalid("Unknown download profile"));
    }
    if interval > 8760 || max == Some(0) {
        return Err(CoreError::invalid("Invalid playlist interval or limit"));
    }
    Ok(())
}

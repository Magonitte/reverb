//! Followed discographies and missing recordings, implemented from E5.
use crate::import::{ImportService, ImportSource, ImportedItem, ImportedTrack};
use crate::{CoreError, CoreResult, Db, EventSink};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ArtistOptions {
    pub monitor_existing: String,
    pub monitor_new: String,
    pub types: Vec<String>,
    pub exclude_variants: bool,
    pub profile_id: String,
    pub output_dir: Option<String>,
}
impl Default for ArtistOptions {
    fn default() -> Self {
        Self {
            monitor_existing: "latest".into(),
            monitor_new: "notify".into(),
            types: vec!["album".into(), "ep".into(), "single".into()],
            exclude_variants: true,
            profile_id: "original".into(),
            output_dir: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ArtistHit {
    pub id: String,
    pub name: String,
    pub picture: Option<String>,
    pub fans: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FollowedArtist {
    pub id: String,
    pub provider_artist_id: String,
    pub name: String,
    pub picture: Option<String>,
    pub options: ArtistOptions,
    #[ts(type = "number | null")]
    pub last_check_at: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ArtistRelease {
    pub artist_id: String,
    pub id: String,
    pub title: String,
    pub record_type: String,
    pub release_date: Option<String>,
    pub cover: Option<String>,
    pub monitored: bool,
    pub tracks: Vec<ImportedTrack>,
    pub present: u32,
    pub total: u32,
    pub state: String,
}

pub fn include_release(
    title: &str,
    kind: &str,
    options: &ArtistOptions,
    base_titles: &HashSet<String>,
) -> bool {
    if !options.types.iter().any(|t| t == kind) {
        return false;
    }
    if !options.exclude_variants {
        return true;
    }
    let base_title = crate::metadata::normalize::norm_plain(
        title.split(['(', '[']).next().unwrap_or(title).trim(),
    );
    let title = crate::metadata::normalize::norm_plain(title);
    if kind == "compile"
        || ["live", "ao vivo", "remix", "remixes"]
            .iter()
            .any(|w| format!(" {title} ").contains(&format!(" {w} ")))
    {
        return false;
    }
    if ["deluxe", "anniversary", "remaster", "remastered"]
        .iter()
        .any(|w| title.contains(w))
        && base_titles.contains(&base_title)
    {
        return false;
    }
    true
}
pub fn recording_present(track: &ImportedTrack, library: &[crate::library::LibraryItem]) -> bool {
    library.iter().filter(|i| !i.missing).any(|i| {
        if let (Some(a), Some(b)) = (&track.isrc, &i.isrc) {
            return a == b;
        }
        crate::metadata::normalize::norm(&track.fields.title)
            == crate::metadata::normalize::norm(&i.title)
            && crate::metadata::normalize::norm(track.fields.artist.as_deref().unwrap_or(""))
                == crate::metadata::normalize::norm(i.artist.as_deref().unwrap_or(""))
            && track
                .duration_s
                .zip(i.duration_s)
                .is_some_and(|(a, b)| (a - b).abs() <= 3.)
    })
}
pub fn target_eligible(item: &crate::library::LibraryItem, target: u32, automatic: bool) -> bool {
    automatic
        && target > 0
        && item.provider.as_deref() == Some("youtube")
        && !item.missing
        && item
            .source_abr_kbps
            .is_some_and(|n| n.is_finite() && n < f64::from(target))
}

pub struct ArtistService {
    db: Db,
    imports: Arc<ImportService>,
    sink: Arc<dyn EventSink>,
    ops: tokio::sync::Mutex<()>,
    clock: Arc<dyn Fn() -> i64 + Send + Sync>,
}
impl ArtistService {
    pub async fn upgrade_tick(
        &self,
        backend: &dyn crate::backend::DownloadBackend,
    ) -> CoreResult<Vec<crate::queue::Job>> {
        let settings = self.imports.settings.get();
        let ids = self
            .library()
            .await?
            .iter()
            .filter(|i| target_eligible(i, settings.quality_target_kbps, settings.auto_upgrade))
            .map(|i| i.id)
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let candidates = crate::quality::upgrade::scan(
            &self.db,
            backend,
            Some(ids),
            &tokio_util::sync::CancellationToken::new(),
        )
        .await?;
        let existing = self.imports.queue.list().await?;
        let mut jobs = Vec::new();
        for c in candidates {
            if existing.iter().any(|j| {
                j.options.upgrade_library_id == Some(c.item.id)
                    && matches!(
                        j.status,
                        crate::queue::JobStatus::Queued | crate::queue::JobStatus::Running
                    )
            }) {
                continue;
            }
            jobs.push(
                self.imports
                    .queue
                    .enqueue(crate::queue::EnqueueRequest {
                        url: c
                            .item
                            .source_url
                            .ok_or_else(|| CoreError::invalid("Source missing"))?,
                        source_id: c.item.source_id,
                        title: Some(c.item.title),
                        profile_id: c.item.profile_id,
                        options: Some(crate::queue::JobOptions {
                            upgrade_library_id: Some(c.item.id),
                            ..Default::default()
                        }),
                        allow_duplicate: true,
                        ..Default::default()
                    })
                    .await?,
            );
        }
        Ok(jobs)
    }
    pub fn start(
        self: &Arc<Self>,
        cancel: tokio_util::sync::CancellationToken,
        backend: Arc<dyn crate::backend::DownloadBackend>,
    ) {
        let service = self.clone();
        tokio::spawn(async move {
            tokio::select! {_=cancel.cancelled()=>return,_=tokio::time::sleep(std::time::Duration::from_secs(300))=>{}}
            loop {
                if !service.imports.settings.get().offline_mode {
                    let now = (service.clock)();
                    let interval =
                        i64::from(service.imports.settings.get().artist_check_interval_hours)
                            * 3600;
                    let work = async {
                        if service
                            .followed()
                            .await?
                            .iter()
                            .any(|a| a.last_check_at.is_none_or(|last| now - last >= interval))
                        {
                            service.check(None).await?;
                        }
                        let last = service
                            .db
                            .kv_get("artists.quality-check")
                            .await?
                            .and_then(|s| s.parse::<i64>().ok());
                        if last.is_none_or(|last| now - last >= 86400) {
                            service.upgrade_tick(backend.as_ref()).await?;
                            service
                                .db
                                .kv_set("artists.quality-check", &now.to_string())
                                .await?;
                        }
                        CoreResult::Ok(())
                    }
                    .await;
                    if let Err(error) = work {
                        tracing::warn!(kind = error.kind(), "Collection maintenance failed");
                    }
                }
                tokio::select! {_=cancel.cancelled()=>return,_=tokio::time::sleep(std::time::Duration::from_secs(900))=>{}}
            }
        });
    }
    pub fn new(db: Db, imports: Arc<ImportService>, sink: Arc<dyn EventSink>) -> Arc<Self> {
        Self::with_clock(db, imports, sink, Arc::new(crate::queue::repo::now))
    }
    pub fn with_clock(
        db: Db,
        imports: Arc<ImportService>,
        sink: Arc<dyn EventSink>,
        clock: Arc<dyn Fn() -> i64 + Send + Sync>,
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            imports,
            sink,
            ops: tokio::sync::Mutex::new(()),
            clock,
        })
    }
    fn changed(&self) {
        self.sink.emit("artists://updated", serde_json::json!({}));
    }
    pub async fn search(&self, name: &str) -> CoreResult<Vec<ArtistHit>> {
        let mut url = url::Url::parse(&format!("{}/search/artist", self.imports.deezer.base))
            .map_err(|_| CoreError::invalid("Invalid endpoint"))?;
        url.query_pairs_mut()
            .append_pair("q", name)
            .append_pair("limit", "5");
        let v = self.imports.deezer.get(url.as_str()).await?;
        v["data"]
            .as_array()
            .ok_or_else(|| CoreError::coded("import_response", "Missing artist results"))?
            .iter()
            .map(hit)
            .collect()
    }
    pub async fn followed(&self) -> CoreResult<Vec<FollowedArtist>> {
        self.db.call(|conn|{
            let mut stmt=conn.prepare("SELECT id,provider_artist_id,name,picture,monitor_existing,monitor_new,types_json,exclude_variants,profile_id,output_dir,last_check_at FROM followed_artists ORDER BY name")?;
            let rows=stmt.query_map([],|r|{let json:String=r.get(6)?;let types=serde_json::from_str(&json).map_err(|e|rusqlite::Error::FromSqlConversionFailure(6,rusqlite::types::Type::Text,Box::new(e)))?;Ok(FollowedArtist{id:r.get(0)?,provider_artist_id:r.get(1)?,name:r.get(2)?,picture:r.get(3)?,options:ArtistOptions{monitor_existing:r.get(4)?,monitor_new:r.get(5)?,types,exclude_variants:r.get(7)?,profile_id:r.get(8)?,output_dir:r.get(9)?},last_check_at:r.get(10)?})})?.collect::<Result<Vec<_>,_>>()?;Ok(rows)
        }).await
    }
    pub async fn follow(
        &self,
        provider_id: &str,
        options: ArtistOptions,
    ) -> CoreResult<FollowedArtist> {
        validate(&options)?;
        if provider_id.is_empty() || !provider_id.bytes().all(|b| b.is_ascii_digit()) {
            return Err(CoreError::invalid("Invalid artist ID"));
        }
        let h = hit(&self
            .imports
            .deezer
            .get(&format!("/artist/{provider_id}"))
            .await?)?;
        let id = uuid::Uuid::new_v4().to_string();
        let saved = id.clone();
        let now = (self.clock)();
        self.db.call(move|conn|{conn.execute("INSERT INTO followed_artists(id,provider_artist_id,name,picture,monitor_existing,monitor_new,types_json,exclude_variants,profile_id,output_dir,created_at) VALUES(?,?,?,?,?,?,?,?,?,?,?)",params![saved,h.id,h.name,h.picture,options.monitor_existing,options.monitor_new,serde_json::to_string(&options.types)?,options.exclude_variants,options.profile_id,options.output_dir,now])?;Ok(())}).await?;
        self.changed();
        self.followed()
            .await?
            .into_iter()
            .find(|a| a.id == id)
            .ok_or_else(|| CoreError::invalid("Artist disappeared"))
    }
    pub async fn update(&self, id: &str, options: ArtistOptions) -> CoreResult<()> {
        validate(&options)?;
        let id = id.to_owned();
        self.db.call(move|conn|{crate::sync::repo::ensure_changed(conn.execute("UPDATE followed_artists SET monitor_existing=?,monitor_new=?,types_json=?,exclude_variants=?,profile_id=?,output_dir=? WHERE id=?",params![options.monitor_existing,options.monitor_new,serde_json::to_string(&options.types)?,options.exclude_variants,options.profile_id,options.output_dir,id])?)?;Ok(())}).await?;
        self.changed();
        Ok(())
    }
    pub async fn unfollow(&self, id: &str, delete_files: bool) -> CoreResult<()> {
        let _ops = self.ops.lock().await;
        let id = id.to_owned();
        self.db
            .call(move |conn| {
                if delete_files {
                    let mut stmt = conn
                        .prepare("SELECT tracks_json FROM followed_releases WHERE artist_id=?")?;
                    let jsons = stmt
                        .query_map([&id], |r| r.get::<_, Option<String>>(0))?
                        .collect::<Result<Vec<_>, _>>()?;
                    let mut ids = Vec::new();
                    for json in jsons.into_iter().flatten() {
                        for t in serde_json::from_str::<Vec<ImportedTrack>>(&json)? {
                            if let Some(isrc) = t.isrc {
                                let mut s = conn.prepare("SELECT id FROM library WHERE isrc=?")?;
                                ids.extend(
                                    s.query_map([isrc], |r| r.get::<_, i64>(0))?
                                        .collect::<Result<Vec<_>, _>>()?,
                                );
                            }
                        }
                    }
                    drop(stmt);
                    ids.sort_unstable();
                    ids.dedup();
                    crate::sync::files::delete_tracks(conn, &ids)?;
                }
                crate::sync::repo::ensure_changed(
                    conn.execute("DELETE FROM followed_artists WHERE id=?", [id])?,
                )?;
                Ok(())
            })
            .await?;
        self.changed();
        Ok(())
    }
    async fn library(&self) -> CoreResult<Vec<crate::library::LibraryItem>> {
        self.db
            .call(|conn| {
                let mut s = conn.prepare("SELECT id FROM library WHERE missing=0")?;
                let ids = s
                    .query_map([], |r| r.get::<_, i64>(0))?
                    .collect::<Result<Vec<_>, _>>()?;
                ids.into_iter()
                    .map(|id| {
                        crate::library::get(conn, id)?
                            .ok_or_else(|| CoreError::invalid("Missing library item"))
                    })
                    .collect()
            })
            .await
    }
    pub async fn releases(&self, id: Option<String>) -> CoreResult<Vec<ArtistRelease>> {
        let library = self.library().await?;
        self.db.call(move|conn|{let mut s=conn.prepare("SELECT artist_id,provider_album_id,title,record_type,release_date,cover,monitored,tracks_json FROM followed_releases WHERE (? IS NULL OR artist_id=?) ORDER BY release_date DESC,title")?;
            let rows=s.query_map(params![id,id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,Option<String>>(4)?,r.get::<_,Option<String>>(5)?,r.get::<_,bool>(6)?,r.get::<_,Option<String>>(7)?)))?.collect::<Result<Vec<_>,_>>()?;
            rows.into_iter().map(|(artist_id,id,title,record_type,release_date,cover,monitored,json)|{let tracks:Vec<ImportedTrack>=json.map(|s|serde_json::from_str(&s)).transpose()?.unwrap_or_default();let total=tracks.len() as u32;let present=tracks.iter().filter(|t|recording_present(t,&library)).count() as u32;let state=if !monitored{"unmonitored"}else if total>0&&present==total{"complete"}else if present>0{"incomplete"}else{"absent"}.into();Ok(ArtistRelease{artist_id,id,title,record_type,release_date,cover,monitored,tracks,total,present,state})}).collect()
        }).await
    }
    pub async fn missing(&self, id: Option<String>) -> CoreResult<Vec<ArtistRelease>> {
        Ok(self
            .releases(id)
            .await?
            .into_iter()
            .filter(|r| r.monitored && r.present < r.total)
            .collect())
    }
    pub async fn download_missing(
        &self,
        release_ids: Vec<String>,
    ) -> CoreResult<Vec<crate::queue::Job>> {
        let library = self.library().await?;
        let artists = self.followed().await?;
        let mut jobs = Vec::new();
        let mut seen = HashSet::new();
        let mut releases = self.releases(None).await?;
        releases.sort_by_key(|r| match r.record_type.as_str() {
            "album" => 0,
            "ep" => 1,
            _ => 2,
        });
        for r in releases.into_iter().filter(|r| release_ids.contains(&r.id)) {
            let artist = artists
                .iter()
                .find(|a| a.id == r.artist_id)
                .ok_or_else(|| CoreError::invalid("Unknown artist"))?;
            for (position, track) in r.tracks.iter().enumerate() {
                let key = track
                    .isrc
                    .clone()
                    .unwrap_or_else(|| format!("{}:{}", r.id, track.id));
                if recording_present(track, &library) || !seen.insert(key) {
                    continue;
                }
                let matched = self.imports.match_track(track).await?;
                if matched.bucket != "ok" {
                    continue;
                }
                if let Some(job) = self
                    .imports
                    .enqueue_item(
                        &ImportedItem {
                            track: track.clone(),
                            matched,
                        },
                        &artist.options.profile_id,
                        artist.options.output_dir.clone(),
                        Some(crate::queue::PlaylistCtx {
                            playlist_title: r.title.clone(),
                            playlist_id: r.id.clone(),
                            index: position as u32 + 1,
                            sync_id: None,
                        }),
                    )
                    .await?
                {
                    jobs.push(job);
                }
            }
        }
        Ok(jobs)
    }
    pub async fn check(&self, max_tracks: Option<usize>) -> CoreResult<Vec<crate::queue::Job>> {
        let _ops = self.ops.lock().await;
        let mut downloads = Vec::new();
        let now = (self.clock)();
        for artist in self.followed().await? {
            let mut albums = self
                .imports
                .deezer
                .pages(&format!(
                    "/artist/{}/albums?limit=100",
                    artist.provider_artist_id
                ))
                .await?;
            let bases = albums
                .iter()
                .filter_map(|v| v["title"].as_str())
                .filter(|s| !s.contains('(') && !s.contains('['))
                .map(crate::metadata::normalize::norm_plain)
                .collect::<HashSet<_>>();
            albums.retain(|v| {
                include_release(
                    v["title"].as_str().unwrap_or(""),
                    v["record_type"].as_str().unwrap_or(""),
                    &artist.options,
                    &bases,
                )
            });
            albums.sort_by(|a, b| b["release_date"].as_str().cmp(&a["release_date"].as_str()));
            let previous = self.releases(Some(artist.id.clone())).await?;
            for (index, album) in albums.iter().enumerate() {
                let album_id = album["id"]
                    .as_u64()
                    .ok_or_else(|| CoreError::coded("import_response", "Missing album ID"))?
                    .to_string();
                let old = previous.iter().find(|r| r.id == album_id);
                let date = album["release_date"].as_str().map(str::to_owned);
                let newly_released = artist.last_check_at.is_some_and(|last| {
                    date.as_ref().is_some_and(|d| {
                        chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")
                            .ok()
                            .and_then(|d| d.and_hms_opt(0, 0, 0))
                            .is_some_and(|d| d.and_utc().timestamp() > last)
                    })
                });
                let action = if artist.last_check_at.is_none() {
                    match artist.options.monitor_existing.as_str() {
                        "all" => "all",
                        "latest" if index == 0 => "all",
                        _ => "none",
                    }
                } else if old.is_none() && newly_released {
                    artist.options.monitor_new.as_str()
                } else {
                    "none"
                };
                let monitored = old.is_some_and(|r| r.monitored) || action != "none";
                let tracks = if let Some(old) = old {
                    old.tracks.clone()
                } else {
                    self.imports
                        .deezer
                        .fetch(&format!("https://www.deezer.com/album/{album_id}"))
                        .await?
                        .tracks
                };
                let tracks = if let Some(max) = max_tracks {
                    tracks.into_iter().take(max).collect()
                } else {
                    tracks
                };
                let id = artist.id.clone();
                let title = album["title"].as_str().unwrap_or("").to_owned();
                let kind = album["record_type"].as_str().unwrap_or("album").to_owned();
                let cover = album["cover_xl"].as_str().map(str::to_owned);
                let saved_album = album_id.clone();
                self.db.call(move|conn|{conn.execute("INSERT INTO followed_releases(artist_id,provider_album_id,title,record_type,release_date,cover,monitored,tracks_json,first_seen_at) VALUES(?,?,?,?,?,?,?,?,?) ON CONFLICT(artist_id,provider_album_id) DO UPDATE SET monitored=excluded.monitored,tracks_json=excluded.tracks_json",params![id,saved_album,title,kind,date,cover,monitored,serde_json::to_string(&tracks)?,now])?;Ok(())}).await?;
                if action == "all" {
                    self.db
                        .kv_set(&format!("artists.auto.{}.{album_id}", artist.id), "true")
                        .await?;
                    downloads.push(album_id.clone());
                } else if monitored
                    && self
                        .db
                        .kv_get(&format!("artists.auto.{}.{album_id}", artist.id))
                        .await?
                        .as_deref()
                        == Some("true")
                {
                    downloads.push(album_id.clone());
                }
                if action == "notify" {
                    self.sink.emit("notice",serde_json::json!({"level":"info","i18nKey":"artists.newRelease","args":{"artist":artist.name,"title":album["title"]}}));
                    self.sink.emit(
                        "artists://release",
                        serde_json::json!({"artist":artist.name,"title":album["title"]}),
                    );
                }
            }
            let id = artist.id.clone();
            self.db
                .call(move |conn| {
                    conn.execute(
                        "UPDATE followed_artists SET last_check_at=? WHERE id=?",
                        params![now, id],
                    )?;
                    Ok(())
                })
                .await?;
        }
        let jobs = self.download_missing(downloads).await?;
        self.changed();
        Ok(jobs)
    }
}
fn hit(v: &serde_json::Value) -> CoreResult<ArtistHit> {
    Ok(ArtistHit {
        id: v["id"]
            .as_u64()
            .ok_or_else(|| CoreError::coded("import_response", "Missing artist ID"))?
            .to_string(),
        name: v["name"]
            .as_str()
            .ok_or_else(|| CoreError::coded("import_response", "Missing artist name"))?
            .into(),
        picture: v["picture_xl"].as_str().map(str::to_owned),
        fans: v["nb_fan"].as_u64().unwrap_or(0) as u32,
    })
}
fn validate(o: &ArtistOptions) -> CoreResult<()> {
    if !matches!(o.monitor_existing.as_str(), "all" | "latest" | "none")
        || !matches!(o.monitor_new.as_str(), "all" | "notify" | "none")
        || o.types.is_empty()
        || o.types
            .iter()
            .any(|t| !matches!(t.as_str(), "album" | "ep" | "single" | "compile"))
        || crate::profiles::profile(&o.profile_id).is_none()
    {
        return Err(CoreError::invalid("Invalid artist options"));
    }
    Ok(())
}

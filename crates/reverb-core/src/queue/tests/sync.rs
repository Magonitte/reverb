//! F11 integration tests use the actual queue, files and SQLite; only source analysis/download are fake.
use super::*;
use crate::sync::{SyncCreate, SyncService, SyncUpdate};
use crate::ytdlp::{CollectionInfo, VideoInfo};
use tokio_util::sync::CancellationToken;

const COLLECTION: &str = "https://www.youtube.com/playlist?list=OLAK5uy_test";

fn collection() -> CollectionInfo {
    CollectionInfo::from_json(&metadata::read("ytdlp/fx4-album.json")).unwrap()
}
fn create(max: Option<u32>) -> SyncCreate {
    SyncCreate {
        url: COLLECTION.into(),
        title: None,
        profile_id: "original".into(),
        output_dir: None,
        interval_hours: 24,
        max_items: max,
        remove_deleted: false,
        write_m3u: true,
    }
}
fn edit(sync: &crate::sync::Sync) -> SyncUpdate {
    SyncUpdate {
        title: sync.title.clone(),
        profile_id: sync.profile_id.clone(),
        output_dir: sync.output_dir.clone(),
        interval_hours: sync.interval_hours,
        max_items: sync.max_items,
        remove_deleted: sync.remove_deleted,
        write_m3u: sync.write_m3u,
        enabled: sync.enabled,
    }
}
fn register(env: &Env, info: &CollectionInfo) {
    env.backend.set_collection(COLLECTION, info.clone());
    for entry in &info.entries {
        let mut video = VideoInfo::from_json(&metadata::read("ytdlp/fx2-music.json")).unwrap();
        video.id = entry.id.clone();
        video.title = entry.title.clone().unwrap_or(entry.id.clone());
        video.track = Some(video.title.clone());
        video.webpage_url = Some(format!("https://www.youtube.com/watch?v={}", entry.id));
        env.backend.set_video(video);
    }
}
async fn setup() -> (Env, QueueService, Arc<SyncService>) {
    let env = Env::without_service(Fix::Never).await;
    env.set(json!({"offlineMode":true,"fetchMetadata":false,"fetchArtwork":false,"fetchLyrics":false,"normalizeVolume":false,"playlistPacingSeconds":0,"maxAttempts":1})).await;
    let server = postprocess::server(false).await;
    let queue = postprocess::start(&env, &server, false).await;
    queue.pause().await;
    let service = SyncService::with_clock(
        env.db.clone(),
        env.settings.clone(),
        env.backend.clone(),
        queue.clone(),
        env.sink.clone(),
        {
            let clock = env.clock.clone();
            Arc::new(move || clock.load(Ordering::SeqCst) as i64)
        },
    );
    register(&env, &collection());
    (env, queue, service)
}
async fn settle(queue: &QueueService, service: &SyncService) {
    queue.resume().await;
    for job in queue.list().await.unwrap() {
        let job = metadata::wait_done(queue, &job.id).await;
        assert!(job.status.is_finished(), "{:?}", job.error_message);
    }
    service.reconcile().await.unwrap();
}

#[tokio::test]
async fn t2_reuses_library_diff_removes_and_trashes_only_when_enabled() {
    let (env, queue, service) = setup().await;
    let info = collection();
    let first = info.entries[0].id.clone();
    let existing = env.dir.path().join("existing.opus");
    std::fs::write(&existing, b"existing audio").unwrap();
    let stored = existing.to_string_lossy().into_owned();
    env.db.call(move|conn| {conn.execute("INSERT INTO library(file_path,provider,source_id,profile_id,title,added_at,updated_at) VALUES(?,'youtube',?,'original','Existing',1,1)",rusqlite::params![stored,first])?;Ok(())}).await.unwrap();
    let sync = service.create(create(None)).await.unwrap();
    let first = service.run(&sync.id).await.unwrap();
    assert_eq!(first.added, 10);
    assert_eq!(queue.list().await.unwrap().len(), 9);
    assert_eq!(
        service
            .items(&sync.id)
            .await
            .unwrap()
            .iter()
            .filter(|item| item.library_id.is_some())
            .count(),
        1
    );
    settle(&queue, &service).await;
    assert_eq!(
        service.list().await.unwrap()[0]
            .last_result
            .as_ref()
            .unwrap()
            .failed,
        0
    );
    let second = service.run(&sync.id).await.unwrap();
    assert_eq!(second.added, 0);
    assert_eq!(queue.list().await.unwrap().len(), 9);
    let mut changed = info.clone();
    changed.entries.remove(0);
    let mut extra = changed.entries[0].clone();
    extra.id = "new00000001".into();
    extra.title = Some("Nova música".into());
    changed.entries.push(extra);
    register(&env, &changed);
    queue.pause().await;
    let next = service.run(&sync.id).await.unwrap();
    assert_eq!((next.added, next.removed), (1, 1));
    assert!(existing.exists());
    assert_eq!(queue.list().await.unwrap().len(), 10);
    settle(&queue, &service).await;
    let mut update = edit(&service.list().await.unwrap()[0]);
    update.remove_deleted = true;
    service.update(&sync.id, update).await.unwrap();
    let removed = service
        .items(&sync.id)
        .await
        .unwrap()
        .into_iter()
        .find(|item| item.source_id == changed.entries[0].id)
        .unwrap();
    let removed_path = PathBuf::from(removed.file_path.unwrap());
    assert!(removed_path.is_file());
    changed.entries.remove(0);
    register(&env, &changed);
    service.run(&sync.id).await.unwrap();
    assert!(!removed_path.exists());
    let removed_id = removed.library_id.unwrap();
    assert!(env
        .db
        .call(move |conn| crate::library::get(conn, removed_id))
        .await
        .unwrap()
        .is_none());
    assert!(existing.exists());
    queue.shutdown().await;
}

#[tokio::test]
async fn t5b_reorder_and_title_rename_audio_and_lyrics_without_download() {
    let (env, queue, service) = setup().await;
    env.set(json!({"fileTemplate":"{playlist}/{playlist_index:03} - {title}"}))
        .await;
    let mut info = collection();
    info.entries.truncate(2);
    register(&env, &info);
    let sync = service.create(create(None)).await.unwrap();
    service.run(&sync.id).await.unwrap();
    settle(&queue, &service).await;
    let first = service.items(&sync.id).await.unwrap()[0].clone();
    let old = PathBuf::from(first.file_path.clone().unwrap());
    let bytes = std::fs::read(&old).unwrap();
    std::fs::write(old.with_extension("lrc"), "[00:00.00]Letra").unwrap();
    let before = env.backend.order().len();
    info.entries.swap(0, 1);
    info.title = Some("Título novo".into());
    register(&env, &info);
    service.run(&sync.id).await.unwrap();
    let moved = service
        .items(&sync.id)
        .await
        .unwrap()
        .into_iter()
        .find(|item| item.source_id == first.source_id)
        .unwrap();
    let path = PathBuf::from(moved.file_path.unwrap());
    assert!(path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("002 - "));
    assert!(path.parent().unwrap().ends_with("Título novo"));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert!(!old.exists());
    assert_eq!(
        std::fs::read_to_string(path.with_extension("lrc")).unwrap(),
        "[00:00.00]Letra"
    );
    assert_eq!(env.backend.order().len(), before);
    let m3u = env.out_dir().join("Playlists/Título novo.m3u8");
    assert_eq!(
        std::fs::read_to_string(m3u)
            .unwrap()
            .matches("#EXTINF:")
            .count(),
        2
    );
    queue.shutdown().await;
}

#[tokio::test]
async fn t5b_rename_collision_keeps_destination_and_moves_lyrics_without_download() {
    let (env, queue, service) = setup().await;
    env.set(json!({"fileTemplate":"{playlist}/{playlist_index:03} - {title}"}))
        .await;
    let mut info = collection();
    info.entries.truncate(1);
    register(&env, &info);
    let sync = service.create(create(None)).await.unwrap();
    service.run(&sync.id).await.unwrap();
    settle(&queue, &service).await;
    let item = service.items(&sync.id).await.unwrap().remove(0);
    let old = PathBuf::from(item.file_path.unwrap());
    let target = env.out_dir().join("Renamed").join(old.file_name().unwrap());
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    let original = std::fs::read(&old).unwrap();
    let destination = b"Existing destination must survive";
    std::fs::write(&target, destination).unwrap();
    std::fs::write(old.with_extension("lrc"), "[00:00.00]Original lyrics").unwrap();
    let mut update = edit(&sync);
    update.title = "Renamed".into();
    service.update(&sync.id, update).await.unwrap();
    assert!(!old.exists());
    assert_ne!(original, destination);
    assert_eq!(std::fs::read(&target).unwrap(), destination);
    assert_eq!(
        std::fs::read_to_string(target.with_extension("lrc")).unwrap(),
        "[00:00.00]Original lyrics"
    );
    let moved = service.items(&sync.id).await.unwrap().remove(0);
    assert_eq!(
        PathBuf::from(moved.file_path.unwrap()),
        crate::library::files::canonical(&target).unwrap()
    );
    assert_eq!(env.backend.order().len(), 1);
    queue.shutdown().await;
}

#[tokio::test]
async fn t5b_duplicate_and_failed_item_retry_on_next_execution() {
    let (env, queue, service) = setup().await;
    let mut info = collection();
    info.entries.truncate(1);
    info.entries.push(info.entries[0].clone());
    register(&env, &info);
    let url = format!("https://www.youtube.com/watch?v={}", info.entries[0].id);
    env.backend
        .script(&url, Script::fail(ErrorKind::Unavailable));
    let sync = service.create(create(None)).await.unwrap();
    let result = service.run(&sync.id).await.unwrap();
    assert_eq!(result.duplicates, vec![info.entries[0].id.clone()]);
    assert_eq!(queue.list().await.unwrap().len(), 1);
    settle(&queue, &service).await;
    assert_eq!(
        service.list().await.unwrap()[0]
            .last_result
            .as_ref()
            .unwrap()
            .failed,
        1
    );
    env.backend.script(&url, Script::ok(10));
    queue.pause().await;
    service.run(&sync.id).await.unwrap();
    assert_eq!(queue.list().await.unwrap().len(), 2);
    settle(&queue, &service).await;
    assert_eq!(env.backend.call_count(&url), 2);
    assert_eq!(
        service.list().await.unwrap()[0]
            .last_result
            .as_ref()
            .unwrap()
            .failed,
        0
    );
    assert!(service.items(&sync.id).await.unwrap()[0]
        .library_id
        .is_some());
    queue.shutdown().await;
}

#[tokio::test]
async fn t4_injected_clock_interval_manual_and_disabled() {
    let (env, queue, service) = setup().await;
    let mut info = collection();
    info.entries.clear();
    register(&env, &info);
    let sync = service.create(create(None)).await.unwrap();
    service.scheduled_tick().await.unwrap();
    let analyses = env.backend.analyze_log().len();
    env.clock.fetch_add(86399, Ordering::SeqCst);
    service.scheduled_tick().await.unwrap();
    assert_eq!(env.backend.analyze_log().len(), analyses);
    env.clock.fetch_add(1, Ordering::SeqCst);
    service.scheduled_tick().await.unwrap();
    assert_eq!(env.backend.analyze_log().len(), analyses + 1);
    let mut update = edit(&service.list().await.unwrap()[0]);
    update.interval_hours = 0;
    service.update(&sync.id, update).await.unwrap();
    env.clock.fetch_add(86400, Ordering::SeqCst);
    service.scheduled_tick().await.unwrap();
    assert_eq!(env.backend.analyze_log().len(), analyses + 1);
    let mut update = edit(&service.list().await.unwrap()[0]);
    update.interval_hours = 24;
    update.enabled = false;
    service.update(&sync.id, update).await.unwrap();
    service.scheduled_tick().await.unwrap();
    assert_eq!(env.backend.analyze_log().len(), analyses + 1);
    queue.shutdown().await;
}

#[tokio::test(start_paused = true)]
async fn t4_background_waits_two_minutes_then_polls_every_fifteen() {
    let env = Env::new().await;
    let mut info = collection();
    info.entries.clear();
    register(&env, &info);
    let service = SyncService::with_clock(
        env.db.clone(),
        env.settings.clone(),
        env.backend.clone(),
        env.queue.clone(),
        env.sink.clone(),
        {
            let clock = env.clock.clone();
            Arc::new(move || clock.load(Ordering::SeqCst) as i64)
        },
    );
    service.create(create(None)).await.unwrap();
    let cancel = CancellationToken::new();
    let started = tokio::time::Instant::now();
    service.start(cancel.clone());
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(119)).await;
    service.reconcile().await.unwrap();
    assert_eq!(env.backend.analyze_log().len(), 1);
    tokio::time::advance(Duration::from_secs(1)).await;
    wait_for_analyses(&env, 2).await;
    service.reconcile().await.unwrap();
    assert_eq!(env.backend.analyze_log().len(), 2);
    env.clock.fetch_add(86400, Ordering::SeqCst);
    tokio::time::advance(
        (started + Duration::from_secs(1019))
            .saturating_duration_since(tokio::time::Instant::now()),
    )
    .await;
    service.reconcile().await.unwrap();
    assert_eq!(env.backend.analyze_log().len(), 2);
    tokio::time::advance(Duration::from_secs(1)).await;
    wait_for_analyses(&env, 3).await;
    service.reconcile().await.unwrap();
    assert_eq!(env.backend.analyze_log().len(), 3);
    cancel.cancel();
    env.queue.shutdown().await;
}

async fn wait_for_analyses(env: &Env, count: usize) {
    // Keep the paused runtime ready while SQLite's real worker thread catches up.
    // A fixed number of yields depends on the CI runner's CPU scheduling.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while env.backend.analyze_log().len() < count {
        assert!(
            std::time::Instant::now() < deadline,
            "scheduled analysis did not start"
        );
        tokio::task::yield_now().await;
    }
}

#[tokio::test(start_paused = true)]
async fn t3_playlist_start_pacing_does_not_delay_single_jobs() {
    let env = Env::new().await;
    env.set(json!({"parallelism":2,"playlistPacingSeconds":3}))
        .await;
    env.queue.pause().await;
    let playlist = |n| EnqueueRequest {
        playlist_ctx: Some(PlaylistCtx {
            playlist_title: "Playlist".into(),
            playlist_id: "list".into(),
            index: n,
            sync_id: None,
        }),
        ..request(n)
    };
    let a = env.queue.enqueue(playlist(1)).await.unwrap();
    let b = env.queue.enqueue(playlist(2)).await.unwrap();
    let single = env.queue.enqueue(request(3)).await.unwrap();
    env.queue.resume().await;
    env.wait_status(&a.id, JobStatus::Done).await;
    env.wait_status(&single.id, JobStatus::Done).await;
    env.wait_status(&b.id, JobStatus::Done).await;
    let a = env.backend.calls(&a.source_url)[0];
    let b = env.backend.calls(&b.source_url)[0];
    let single = env.backend.calls(&single.source_url)[0];
    assert!(b.duration_since(a) >= Duration::from_secs(3));
    assert!(single.duration_since(a) < Duration::from_secs(3));
    env.queue.shutdown().await;
}

use super::*;
use reverb_core::sync::{SyncCreate, SyncService};

const FX4: &str = "https://www.youtube.com/playlist?list=OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE";
const FX5: &str = "https://music.youtube.com/browse/MPREb_dcYZhAh5urI";
fn request(url: &str) -> SyncCreate {
    SyncCreate {
        url: url.into(),
        title: None,
        profile_id: "original".into(),
        output_dir: None,
        interval_hours: 0,
        max_items: Some(2),
        remove_deleted: false,
        write_m3u: true,
    }
}
pub(super) async fn service(stack: &Stack) -> (QueueService, Arc<SyncService>) {
    let heal = Arc::new(HealCoordinator::new(
        Arc::new(ToolsHeal::new(stack.tools.clone(), stack.settings.clone())),
        stack.db.clone(),
        stack.sink.clone(),
    ));
    let queue = QueueService::start(QueueDeps {
        db: stack.db.clone(),
        settings: stack.settings.clone(),
        sink: stack.sink.clone(),
        runner: Arc::new(
            ToolsPipeline::new(
                stack.backend.clone(),
                stack.tools.clone(),
                stack.data.clone(),
            )
            .with_metadata(stack.metadata.clone()),
        ),
        heal,
        data_dir: stack.data.clone(),
        start_paused: false,
    })
    .await
    .unwrap();
    let service = SyncService::new(
        stack.db.clone(),
        stack.settings.clone(),
        stack.backend.clone(),
        queue.clone(),
        stack.sink.clone(),
    );
    (queue, service)
}

#[tokio::test]
#[ignore = "network"]
async fn t7_two_real_tracks_relative_playlist_and_idempotent_second_run() {
    let stack = stack().await;
    let (queue, service) = service(&stack).await;
    let sync = service.create(request(FX4)).await.unwrap();
    let run = service.run(&sync.id).await.unwrap();
    assert_eq!(run.added, 2);
    let deadline = Instant::now() + Duration::from_secs(360);
    loop {
        service.reconcile().await.unwrap();
        let current = service.list().await.unwrap().remove(0);
        if !current.last_result.as_ref().unwrap().running {
            assert_eq!(
                current.last_result.as_ref().unwrap().failed,
                0,
                "{:?}",
                current.last_result
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "playlist jobs did not finish: {:?}",
            queue.list().await.unwrap()
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let items = service.items(&sync.id).await.unwrap();
    assert_eq!(items.len(), 2);
    let root = reverb_core::library::files::canonical(&stack._temp.path().join("musicas")).unwrap();
    for item in &items {
        let path = std::path::Path::new(item.file_path.as_ref().unwrap());
        assert!(path.is_file());
        assert!(path.starts_with(&root));
        assert_eq!(path.extension().unwrap(), "opus");
        assert!(std::fs::metadata(path).unwrap().len() > 10000);
    }
    let path =
        reverb_core::organize::sanitize_path(&root.join("Playlists"), &[&sync.title], "m3u8");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("#EXTM3U\n"));
    assert_eq!(text.matches("#EXTINF:").count(), 2);
    for line in text.lines().filter(|line| !line.starts_with('#')) {
        assert!(!std::path::Path::new(line).is_absolute());
        assert!(!line.contains('\\'));
        assert!(path.parent().unwrap().join(line).is_file());
    }
    let jobs = queue.list().await.unwrap().len();
    let again = service.run(&sync.id).await.unwrap();
    assert_eq!((again.added, again.removed, again.failed), (0, 0, 0));
    assert!(!again.running);
    assert_eq!(queue.list().await.unwrap().len(), jobs);
    queue.shutdown().await;
    stack.tools.shutdown().await;
}

#[tokio::test]
#[ignore = "network"]
async fn t8_music_browse_url_has_same_playlist_identity() {
    let stack = stack().await;
    let (queue, service) = service(&stack).await;
    let playlist = service.create(request(FX4)).await.unwrap();
    let browse = service.create(request(FX5)).await.unwrap();
    assert_eq!(playlist.playlist_id, browse.playlist_id);
    assert_eq!(
        playlist.playlist_id.as_deref(),
        Some("OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE")
    );
    queue.shutdown().await;
    stack.tools.shutdown().await;
}

use super::*;
use reverb_core::artists::{ArtistOptions, ArtistService};
use reverb_core::import::ImportService;
async fn imports(stack: &Stack) -> (QueueService, Arc<ImportService>) {
    let (queue, _) = sync::service(stack).await;
    queue.pause().await;
    let service = ImportService::new(
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
async fn f15_t10_chart_playlist_all_isrc_three_matches() {
    let stack = stack().await;
    let (queue, imports) = imports(&stack).await;
    let chart = imports
        .deezer
        .get("/chart/0/playlists?limit=1")
        .await
        .unwrap();
    let id = chart["data"][0]["id"].as_u64().unwrap();
    let collection = imports
        .fetch(&format!("https://deezer.com/playlist/{id}"))
        .await
        .unwrap();
    assert!(!collection.tracks.is_empty());
    assert!(collection.tracks.iter().all(|t| t.isrc.is_some()));
    let mut confident = 0;
    for track in collection.tracks.iter().take(3) {
        let matched = imports.match_track(track).await.unwrap();
        eprintln!("{} => {:?}", track.fields.title, matched);
        if matched.confidence >= 0.85 {
            confident += 1;
        }
    }
    assert!(confident >= 2, "Only {confident}/3 confident matches");
    queue.shutdown().await;
}
#[tokio::test]
#[ignore = "network"]
async fn f15_t11_latest_rick_astley_three_isrc_jobs_and_library() {
    let stack = stack().await;
    stack
        .settings
        .update(
            serde_json::from_value(
                serde_json::json!({"fetchLyrics":false,"playlistPacingSeconds":0}),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let (queue, imports) = imports(&stack).await;
    let artists = ArtistService::new(stack.db.clone(), imports.clone(), stack.sink.clone());
    let followed = artists
        .follow(
            "6160",
            ArtistOptions {
                types: vec!["album".into()],
                monitor_existing: "latest".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let jobs = artists.check(Some(3)).await.unwrap();
    assert_eq!(jobs.len(), 3);
    let release = artists
        .releases(Some(followed.id))
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.monitored)
        .unwrap();
    let mut confident = 0;
    for job in &jobs {
        let metadata = job.metadata_override.as_ref().unwrap();
        let matched: reverb_core::import::MatchResult =
            serde_json::from_value(metadata["importMatch"].clone()).unwrap();
        assert_eq!(
            matched.via,
            reverb_core::metadata::official::OfficialVia::Isrc,
            "{} / {:?}: {:?}; queued jobs: {:?}",
            metadata["title"],
            metadata["isrc"],
            matched,
            jobs.iter()
                .map(|j| (&j.source_id, &j.title))
                .collect::<Vec<_>>()
        );
        if matched.confidence >= 0.85 {
            confident += 1;
        }
    }
    assert!(confident >= 2);
    for j in jobs.iter().skip(1) {
        queue.cancel(&j.id).await.unwrap();
    }
    queue.resume().await;
    let deadline = Instant::now() + Duration::from_secs(360);
    let id = jobs[0].id.clone();
    loop {
        let job = queue
            .list()
            .await
            .unwrap()
            .into_iter()
            .find(|j| j.id == id)
            .unwrap();
        if job.status.is_finished() {
            assert_eq!(job.status, JobStatus::Done, "{:?}", job.error_message);
            let library = stack
                .db
                .call(move |conn| reverb_core::library::get(conn, job.library_id.unwrap()))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(library.isrc, release.tracks[0].isrc);
            assert_eq!(
                reverb_core::tagging::read_tags(std::path::Path::new(&library.file_path))
                    .unwrap()
                    .isrc,
                library.isrc
            );
            break;
        }
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    queue.shutdown().await;
}

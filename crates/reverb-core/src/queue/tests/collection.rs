use super::*;
use crate::artists::{
    include_release, recording_present, target_eligible, ArtistOptions, ArtistService,
};
use crate::import::{DeezerSource, ImportService, SpotifySource};
use crate::ytdlp::{SearchResult, SearchSource, VideoInfo};
use std::collections::HashSet;
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};
fn track(id: u32) -> Value {
    json!({"id":id,"title":format!("Song {id}"),"duration":214,"isrc":format!("GBARL9300{id:03}"),"track_position":id,"disk_number":1,"artist":{"name":"Artist"},"album":{"title":"Source Album","cover_xl":"https://image.example/cover"}})
}
async fn setup() -> (
    Env,
    QueueService,
    Arc<ImportService>,
    Arc<crate::sync::SyncService>,
    MockServer,
) {
    let env = Env::without_service(Fix::Never).await;
    env.set(json!({"fetchArtwork":false,"fetchLyrics":false,"normalizeVolume":false,"playlistPacingSeconds":0})).await;
    let server = postprocess::server(false).await;
    let queue = postprocess::start(&env, &server, false).await;
    queue.pause().await;
    let source = Arc::new(DeezerSource::new(&server.uri()));
    let imports = ImportService::with_sources(
        env.db.clone(),
        env.settings.clone(),
        env.backend.clone(),
        queue.clone(),
        env.sink.clone(),
        source,
        SpotifySource::with_endpoints(env.settings.clone(), &server.uri(), &server.uri()),
    );
    let sync = crate::sync::SyncService::new(
        env.db.clone(),
        env.settings.clone(),
        env.backend.clone(),
        queue.clone(),
        env.sink.clone(),
    );
    sync.with_imports(imports.clone());
    for id in 1..=4 {
        let video_id = format!("import{id:05}");
        let mut v = VideoInfo::from_json(&metadata::read("ytdlp/fx2-music.json")).unwrap();
        v.id = video_id.clone();
        v.title = format!("Song {id}");
        v.track = Some(v.title.clone());
        v.artist = Some("Artist".into());
        v.artists = vec!["Artist".into()];
        v.album = Some("Wrong YouTube Album".into());
        env.backend.set_video(v);
        env.backend.set_search(
            SearchSource::YtMusic,
            &format!("GBARL9300{id:03}"),
            vec![SearchResult {
                id: video_id,
                title: format!("Song {id}"),
                url: None,
                duration: None,
                channel: None,
            }],
        );
    }
    (env, queue, imports, sync, server)
}
#[tokio::test]
async fn external_sync_checksum_diff_and_metadata_priority() {
    let (env, queue, imports, sync, s) = setup().await;
    Mock::given(path("/playlist/1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"title":"Source Playlist","checksum":"one"})),
        )
        .mount(&s)
        .await;
    Mock::given(path("/playlist/1/tracks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[track(1),track(2)]})))
        .mount(&s)
        .await;
    let request = crate::sync::SyncCreate {
        url: "https://deezer.com/playlist/1".into(),
        title: None,
        profile_id: "original".into(),
        output_dir: None,
        interval_hours: 24,
        max_items: None,
        remove_deleted: false,
        write_m3u: false,
    };
    let saved = sync.create(request).await.unwrap();
    assert_eq!(saved.provider, "deezer");
    assert_eq!(sync.run(&saved.id).await.unwrap().added, 2);
    let jobs = queue.list().await.unwrap();
    assert_eq!(jobs.len(), 2);
    queue.resume().await;
    for job in &jobs {
        let finished = metadata::wait_done(&queue, &job.id).await;
        assert_eq!(
            finished.status,
            JobStatus::Done,
            "{:?}",
            finished.error_message
        );
    }
    sync.reconcile().await.unwrap();
    queue.pause().await;
    assert_eq!(
        jobs[0].metadata_override.as_ref().unwrap()["album"],
        "Source Album"
    );
    assert_eq!(
        jobs[0].metadata_override.as_ref().unwrap()["isrc"],
        "GBARL9300001"
    );
    assert_eq!(
        jobs[0].metadata_override.as_ref().unwrap()["importMatch"]["via"],
        "isrc"
    );
    assert_eq!(
        jobs[0].metadata_override.as_ref().unwrap()["importMatch"]["videoId"],
        jobs[0].source_id.as_deref().unwrap()
    );
    let before = env.backend.analyze_log().len();
    assert_eq!(sync.run(&saved.id).await.unwrap().added, 0);
    assert_eq!(env.backend.analyze_log().len(), before);
    s.reset().await;
    Mock::given(path("/playlist/1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"title":"Source Playlist","checksum":"two"})),
        )
        .mount(&s)
        .await;
    Mock::given(path("/playlist/1/tracks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[track(2),track(3)]})))
        .mount(&s)
        .await;
    let result = sync.run(&saved.id).await.unwrap();
    assert_eq!((result.added, result.removed), (1, 1));
    assert_eq!(env.backend.search_log().len(), 3);
    assert!(sync
        .items(&saved.id)
        .await
        .unwrap()
        .iter()
        .any(|i| i.source_id == "1" && i.state == "removed"));
    let matched = imports
        .match_track(&crate::import::deezer_track(&track(4), None).unwrap())
        .await
        .unwrap();
    assert_eq!(matched.confidence, 1.);
    assert_eq!(matched.via, crate::metadata::official::OfficialVia::Isrc);
    queue.shutdown().await;
}
#[test]
fn release_filters_cover_variants_and_type_preferences() {
    let o = ArtistOptions::default();
    let bases = HashSet::from(["album".into()]);
    for (title, kind, want) in [
        ("Album", "album", true),
        ("New EP", "ep", true),
        ("New Song", "single", true),
        ("Collection", "compile", false),
        ("Album (Live)", "album", false),
        ("Ao Vivo", "album", false),
        ("Remixes", "ep", false),
        ("Remix", "single", false),
        ("Album (Deluxe)", "album", false),
        ("Album (Remastered)", "album", false),
        ("Album (Anniversary)", "album", false),
        ("Other (Deluxe)", "album", true),
    ] {
        assert_eq!(include_release(title, kind, &o, &bases), want, "{title}");
    }
}
#[tokio::test]
async fn artist_first_monitoring_new_release_policies_missing_and_numbering() {
    for existing in ["all", "latest", "none"] {
        for new in ["all", "notify", "none"] {
            let (env, queue, imports, _, s) = setup().await;
            Mock::given(path("/artist/6160"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(json!({"id":6160,"name":"Artist"})),
                )
                .mount(&s)
                .await;
            Mock::given(path("/artist/6160/albums")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[{"id":10,"title":"Album","record_type":"album","release_date":"2025-01-01"},{"id":11,"title":"Single","record_type":"single","release_date":"2024-01-01"}]}))).mount(&s).await;
            for id in [10, 11] {
                Mock::given(path(format!("/album/{id}")))
                    .respond_with(ResponseTemplate::new(200).set_body_json(
                        json!({"id":id,"title":if id==10{"Album"}else{"Single"},"nb_tracks":1}),
                    ))
                    .mount(&s)
                    .await;
                Mock::given(path(format!("/album/{id}/tracks")))
                    .respond_with(
                        ResponseTemplate::new(200).set_body_json(json!({"data":[track(1)]})),
                    )
                    .mount(&s)
                    .await;
            }
            let clock = Arc::new(AtomicU64::new(1735689600));
            let service = ArtistService::with_clock(env.db.clone(), imports, env.sink.clone(), {
                let c = clock.clone();
                Arc::new(move || c.load(Ordering::SeqCst) as i64)
            });
            let artist = service
                .follow(
                    "6160",
                    ArtistOptions {
                        monitor_existing: existing.into(),
                        monitor_new: new.into(),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
            let jobs = service.check(None).await.unwrap();
            assert_eq!(jobs.len(), usize::from(existing != "none"));
            if let Some(j) = jobs.first() {
                assert_eq!(j.playlist_ctx.as_ref().unwrap().playlist_title, "Album");
                assert_eq!(j.playlist_ctx.as_ref().unwrap().index, 1);
            }
            let releases = service.releases(Some(artist.id.clone())).await.unwrap();
            assert_eq!(
                releases.iter().filter(|r| r.monitored).count(),
                match existing {
                    "all" => 2,
                    "latest" => 1,
                    _ => 0,
                }
            );
            let missing = service.missing(Some(artist.id.clone())).await.unwrap();
            assert!(missing.iter().all(|r| r.state == "absent"));
            s.reset().await;
            Mock::given(path("/artist/6160/albums")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[{"id":12,"title":"New Album","record_type":"album","release_date":"2025-02-01"}]}))).mount(&s).await;
            Mock::given(path("/album/12"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(json!({"title":"New Album"})),
                )
                .mount(&s)
                .await;
            Mock::given(path("/album/12/tracks"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[track(2)]})))
                .mount(&s)
                .await;
            clock.store(1740787200, Ordering::SeqCst);
            let jobs = service.check(None).await.unwrap();
            assert_eq!(jobs.len(), usize::from(new == "all"));
            let releases = service.releases(Some(artist.id)).await.unwrap();
            assert_eq!(
                releases.iter().find(|r| r.id == "12").unwrap().monitored,
                new != "none"
            );
            queue.shutdown().await;
        }
    }
}
#[tokio::test]
async fn missing_matches_isrc_or_normalized_title_duration_and_quality_never_downgrades() {
    let (env, queue, imports, _, _) = setup().await;
    env.db.call(|conn|{conn.execute("INSERT INTO library(file_path,title,artist,duration_s,isrc,provider,source_id,source_url,source_abr_kbps,profile_id,added_at,updated_at) VALUES('fixture.opus','Song 1','Artist',214,'GBARL9300001','youtube','import00001','https://music.youtube.com/watch?v=import00001',129,'original',1,1)",[])?;Ok(())}).await.unwrap();
    let mut item = env
        .db
        .call(|conn| crate::library::get(conn, 1))
        .await
        .unwrap()
        .unwrap();
    let mut t = crate::import::deezer_track(&track(1), None).unwrap();
    assert!(recording_present(&t, &[item.clone()]));
    t.isrc = None;
    t.fields.title = "Sóng 1".into();
    t.duration_s = Some(216.9);
    assert!(recording_present(&t, &[item.clone()]));
    t.duration_s = Some(218.);
    assert!(!recording_present(&t, &[item.clone()]));
    for (current, target, automatic, want) in [
        (129., 160, true, true),
        (160., 160, true, false),
        (256., 160, true, false),
        (129., 256, false, false),
        (129., 0, true, false),
        (220., 256, true, true),
    ] {
        item.source_abr_kbps = Some(current);
        assert_eq!(target_eligible(&item, target, automatic), want);
    }
    item.source_abr_kbps = Some(220.);
    assert!(!crate::quality::upgrade::eligible(&item, 200.));
    assert!(crate::quality::upgrade::eligible(&item, 270.));
    let artists = ArtistService::new(env.db.clone(), imports, env.sink.clone());
    let expected = [
        crate::import::deezer_track(&track(1), None).unwrap(),
        crate::import::deezer_track(&track(2), None).unwrap(),
        crate::import::deezer_track(&track(3), None).unwrap(),
    ];
    env.db.call(move|conn|{
        conn.execute("INSERT INTO followed_artists(id,provider_artist_id,name,profile_id,created_at) VALUES('artist1','1','Artist','original',1)",[])?;
        for (id,tracks) in [("complete",vec![expected[0].clone()]),("partial",expected[..2].to_vec()),("absent",vec![expected[2].clone()])]{conn.execute("INSERT INTO followed_releases(artist_id,provider_album_id,title,record_type,monitored,tracks_json,first_seen_at) VALUES('artist1',?,?,'album',1,?,1)",rusqlite::params![id,id,serde_json::to_string(&tracks)?])?;}Ok(())
    }).await.unwrap();
    let states = artists.releases(None).await.unwrap();
    assert_eq!(
        states.iter().find(|r| r.id == "complete").unwrap().state,
        "complete"
    );
    let partial = states.iter().find(|r| r.id == "partial").unwrap();
    assert_eq!((partial.present, partial.total), (1, 2));
    assert_eq!(partial.state, "incomplete");
    assert_eq!(
        states.iter().find(|r| r.id == "absent").unwrap().state,
        "absent"
    );
    let mut video = VideoInfo::from_json(&metadata::read("ytdlp/fx2-music.json")).unwrap();
    video.id = "import00001".into();
    video.best_audio_abr = Some(256.);
    env.backend.set_video(video);
    env.set(json!({"qualityTargetKbps":256,"autoUpgrade":false}))
        .await;
    assert!(artists
        .upgrade_tick(env.backend.as_ref())
        .await
        .unwrap()
        .is_empty());
    env.set(json!({"autoUpgrade":true})).await;
    let improved = artists.upgrade_tick(env.backend.as_ref()).await.unwrap();
    assert_eq!(improved.len(), 1);
    assert_eq!(improved[0].options.upgrade_library_id, Some(1));
    assert!(artists
        .upgrade_tick(env.backend.as_ref())
        .await
        .unwrap()
        .is_empty());
    env.db
        .call(|c| {
            c.execute("UPDATE library SET source_abr_kbps=256", [])?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(artists
        .upgrade_tick(env.backend.as_ref())
        .await
        .unwrap()
        .is_empty());
    queue.shutdown().await;
}

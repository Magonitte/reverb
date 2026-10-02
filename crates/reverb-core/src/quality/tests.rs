use super::*;
use crate::metadata::MetadataService;
use crate::queue::fake::FakeBackend;
use crate::tagging::{read_tags, write_tags, TagCover, TrackTags};
use crate::test_tools as common;
use crate::{Db, MemorySink, SettingsService};
use std::path::Path;
use std::sync::Arc;

async fn audio(root: &Path, filter: &str) -> std::path::PathBuf {
    let path = root.join("source.opus");
    audio::execute(
        &common::ffmpeg(),
        vec![
            "-nostdin".into(),
            "-y".into(),
            "-f".into(),
            "lavfi".into(),
            "-i".into(),
            filter.into(),
            "-c:a".into(),
            "libopus".into(),
            path.to_string_lossy().into_owned(),
        ],
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    path
}
fn tagged(path: &Path) -> TrackTags {
    let image = image::DynamicImage::new_rgb8(16, 16);
    let mut png = std::io::Cursor::new(Vec::new());
    image.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let tags = TrackTags {
        title: "Edited title".into(),
        artist: Some("Artist".into()),
        album: Some("Album".into()),
        lyrics: Some("[00:01.00]lyrics".into()),
        genre: Some("Custom".into()),
        cover: Some(TagCover {
            mime_type: "image/png".into(),
            data: png.into_inner(),
        }),
        ..Default::default()
    };
    write_tags(path, &tags).unwrap();
    tags
}
#[tokio::test]
async fn trim_preserves_tags_cover_and_waveform_dimensions() {
    let dir = tempfile::tempdir().unwrap();
    let path = audio(dir.path(), "sine=duration=10").await;
    let tags = tagged(&path);
    let db = Db::open_in_memory().unwrap();
    let original = std::fs::read(&path).unwrap();
    assert!(trim(
        &db,
        &common::ffmpeg(),
        &common::ffprobe(),
        &path,
        4.0,
        1.0,
        &CancellationToken::new()
    )
    .await
    .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let wave = audio::waveform(&common::ffmpeg(), &path, &CancellationToken::new())
        .await
        .unwrap();
    let image = image::load_from_memory(&wave).unwrap();
    assert_eq!((image.width(), image.height()), (1200, 160));
    trim(
        &db,
        &common::ffmpeg(),
        &common::ffprobe(),
        &path,
        1.0,
        4.0,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(read_tags(&path).unwrap(), tags);
    assert!(
        (crate::transcode::probe(&common::ffprobe(), &path)
            .await
            .unwrap()
            .duration_s
            - 3.0)
            .abs()
            < 0.2
    );
}
#[tokio::test]
async fn silence_filter_removes_both_edges() {
    let dir = tempfile::tempdir().unwrap();
    let path = audio(
        dir.path(),
        "aevalsrc=if(between(t\\,2\\,7)\\,0.2*sin(2*PI*440*t)\\,0):d=10:s=48000",
    )
    .await;
    let ready = dir.path().join("trim.opus");
    let profile = crate::profiles::profile("original")
        .unwrap()
        .resolve("opus");
    audio::silence(
        &common::ffmpeg(),
        &path,
        &ready,
        &profile,
        129.0,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    let duration = crate::transcode::probe(&common::ffprobe(), &ready)
        .await
        .unwrap()
        .duration_s;
    assert!((duration - 5.0).abs() < 0.6, "{duration}");
}
#[tokio::test]
async fn atomic_replacement_rolls_back_without_commit() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("song.opus");
    let ready = dir.path().join("new.opus");
    std::fs::write(&target, b"old").unwrap();
    std::fs::write(&ready, b"new").unwrap();
    {
        let _replacement = replace::Replacement::publish(&ready, &target).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
    }
    assert_eq!(std::fs::read(&target).unwrap(), b"old");
}
#[tokio::test]
async fn chapters_create_three_tagged_files_and_clean_titles() {
    let dir = tempfile::tempdir().unwrap();
    let source = audio(dir.path(), "sine=duration=30").await;
    let backend = FakeBackend::new();
    backend.set_audio(std::fs::read(source).unwrap(), false);
    let video=VideoInfo::from_json(r#"{"id":"chaptertest","title":"Artist - Album (Full Album)","duration":601,"channel":"Artist","categories":["Music"],"chapters":[{"title":"01. First","start_time":0,"end_time":10},{"title":"2 - Second","start_time":10,"end_time":20},{"title":"00:20 Third","start_time":20,"end_time":30}]}"#).unwrap();
    backend.set_video(video);
    let db = Db::open_in_memory().unwrap();
    let service = Arc::new(
        SettingsService::new(db, Arc::new(MemorySink::new()))
            .await
            .unwrap(),
    );
    service
        .update(
            serde_json::from_value(
                serde_json::json!({"preferOfficialAudio":false,"fetchMetadata":false}),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let metadata = Arc::new(MetadataService::with_providers(
        backend.clone(),
        service.clone(),
        vec![],
    ));
    let pipeline = crate::pipeline::DownloadPipeline::new(
        backend,
        dir.path().join("data"),
        common::ffmpeg().parent().unwrap().to_owned(),
        None,
    )
    .with_metadata(Some(metadata))
    .with_postprocessing(Arc::new(
        crate::pipeline::postprocess::PostProcessor::default(),
    ));
    let mut settings = service.get();
    settings.fetch_artwork = false;
    settings.fetch_lyrics = false;
    settings.normalize_volume = false;
    settings.split_chapters = crate::settings::SplitChapters::Always;
    let job = crate::pipeline::PipelineJob {
        job_id: "chapters".into(),
        url: "https://youtu.be/chaptertest".into(),
        profile: crate::profiles::profile("original").unwrap(),
        out_dir: dir.path().join("music"),
        sponsorblock: None,
        metadata_override: None,
        fetch_metadata: None,
        settings: Some(settings),
        options: Default::default(),
        playlist_ctx: None,
    };
    let output = pipeline
        .run(&job, &CancellationToken::new(), &|_| {})
        .await
        .unwrap();
    let mut files = vec![output.library_file.as_ref().unwrap()];
    files.extend(output.chapter_files.iter());
    assert_eq!(files.len(), 3);
    for (index, file) in files.into_iter().enumerate() {
        let path = Path::new(&file.file_path);
        let tags = read_tags(path).unwrap();
        assert_eq!(tags.track_no, Some(index as u32 + 1));
        assert_eq!(tags.track_total, Some(3));
        assert_eq!(tags.title, ["First", "Second", "Third"][index]);
        assert!(tags.album.as_ref().unwrap().contains("Album"));
        assert!(
            (crate::transcode::probe(&common::ffprobe(), path)
                .await
                .unwrap()
                .duration_s
                - 10.0)
                .abs()
                < 0.3
        );
    }
}

#[tokio::test]
async fn upgrade_preserves_current_tags_replaces_file_and_updates_database() {
    upgrade_case(false).await;
    upgrade_case(true).await;
}

async fn upgrade_case(chapter: bool) {
    use crate::queue::{
        EnqueueRequest, HealCoordinator, JobOptions, JobStatus, QueueDeps, QueueService, ToolsHeal,
        ToolsPipeline,
    };
    let dir = tempfile::tempdir().unwrap();
    let source = audio(dir.path(), "sine=frequency=440:duration=10").await;
    let mut tags = tagged(&source);
    if chapter {
        tags.track_no = Some(2);
        tags.track_total = Some(2);
        write_tags(&source, &tags).unwrap();
    }
    let original = std::fs::read(&source).unwrap();
    let source = crate::library::files::canonical(&source).unwrap();
    let stored = source.to_string_lossy().into_owned();
    let db = Db::open_in_memory().unwrap();
    db.call(move|conn|{conn.execute("INSERT INTO library(id,file_path,provider,source_id,source_url,profile_id,title,source_abr_kbps,added_at,updated_at) VALUES(1,?,'youtube','upgrade0001','https://youtu.be/upgrade0001','original','Old',129,0,0)",[stored])?;Ok(())}).await.unwrap();
    if chapter {
        db.call(|conn| {
            conn.execute("UPDATE library SET track_no=2,track_total=2 WHERE id=1", [])?;
            Ok(())
        })
        .await
        .unwrap();
    }
    let sink = Arc::new(MemorySink::new());
    let settings = Arc::new(
        SettingsService::new(db.clone(), sink.clone())
            .await
            .unwrap(),
    );
    let backend = FakeBackend::new();
    backend.set_audio(original.clone(), false);
    backend.set_source_abr(256.0);
    backend.set_video(VideoInfo::from_json(if chapter { r#"{"id":"upgrade0001","duration":601,"chapters":[{"start_time":0,"end_time":5,"title":"One"},{"start_time":5,"end_time":10,"title":"Two"}],"formats":[{"format_id":"774","abr":256,"acodec":"opus","vcodec":"none","ext":"opus"}]}"# } else { r#"{"id":"upgrade0001","duration":10,"formats":[{"format_id":"774","abr":256,"acodec":"opus","vcodec":"none","ext":"opus"}]}"# }).unwrap());
    let item = db
        .call(|conn| crate::library::get(conn, 1))
        .await
        .unwrap()
        .unwrap();
    for (current, available, provider, want) in [
        (129.0, 129.0, "youtube", false),
        (129.0, 256.0, "youtube", true),
        (160.0, 192.0, "youtube", false),
        (129.0, 256.0, "deezer", false),
    ] {
        let mut item = item.clone();
        item.source_abr_kbps = Some(current);
        item.provider = Some(provider.into());
        assert_eq!(upgrade::eligible(&item, available), want);
    }
    let tools = Arc::new(
        crate::ToolsManager::new(
            crate::ToolsConfig::new(common::tools_root()),
            db.clone(),
            settings.clone(),
            sink.clone(),
        )
        .unwrap(),
    );
    let runner = Arc::new(
        ToolsPipeline::new(backend, tools.clone(), dir.path().join("data"))
            .with_database(db.clone()),
    );
    let heal = Arc::new(HealCoordinator::new(
        Arc::new(ToolsHeal::new(tools, settings.clone())),
        db.clone(),
        sink.clone(),
    ));
    let queue = QueueService::start(QueueDeps {
        db: db.clone(),
        settings,
        sink,
        runner,
        heal,
        data_dir: dir.path().join("data"),
        start_paused: true,
    })
    .await
    .unwrap();
    let request = EnqueueRequest {
        url: "https://youtu.be/ignored".into(),
        options: Some(JobOptions {
            upgrade_library_id: Some(1),
            ..Default::default()
        }),
        ..Default::default()
    };
    let job = queue.enqueue(request.clone()).await.unwrap();
    assert_eq!(job.kind, "upgrade");
    assert!(queue.enqueue(request).await.is_err());
    queue.resume().await;
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            let job = queue.get(&job.id).await.unwrap().unwrap();
            if job.status.is_finished() {
                break job;
            }
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap();
    queue.shutdown().await;
    assert_eq!(result.status, JobStatus::Done, "{:?}", result.error_message);
    assert_eq!(result.library_id, Some(1));
    let current = read_tags(&source).unwrap();
    assert_eq!(current.title, tags.title);
    assert_eq!(current.genre, tags.genre);
    assert_eq!(current.cover, tags.cover);
    assert_eq!(current.lyrics, tags.lyrics);
    assert!(current.replay_gain_track_gain.is_some());
    assert_ne!(std::fs::read(&source).unwrap(), original);
    let item = db
        .call(|conn| crate::library::get(conn, 1))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(item.source_abr_kbps, Some(256.0));
    assert!(item.bitrate_kbps.is_some());
    assert!((item.duration_s.unwrap() - if chapter { 5.0 } else { 10.0 }).abs() < 0.3);
    assert_eq!(current.track_no, tags.track_no);
    assert!(!std::fs::read_dir(dir.path()).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".reverb-old-")));
}

#[tokio::test]
async fn fingerprint_real_and_acoustid_http_response() {
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let dir = tempfile::tempdir().unwrap();
    let source = audio(dir.path(), "sine=duration=15").await;
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/lookup")).and(query_param("client","key")).and(query_param("meta","recordings+releasegroups")).respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status":"ok","results":[{"id":"sound","score":0.95,"recordings":[{"id":"mb","title":"Song","artists":[{"name":"Artist"}]}]}]}))).mount(&server).await;
    let fpcalc = common::tool_dir("fpcalc").join(common::exe("fpcalc"));
    let candidates = acoustid::lookup(&fpcalc, &source, "key", &format!("{}/lookup", server.uri()))
        .await
        .unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(
        candidates[0].candidate.mb_recording_id.as_deref(),
        Some("mb")
    );
    let requests = server.received_requests().await.unwrap();
    assert!(requests[0]
        .url
        .query_pairs()
        .any(|(k, v)| k == "fingerprint" && !v.is_empty()));
}

#[tokio::test]
async fn duplicate_recording_identity_matches_different_urls_and_profiles() {
    let db = Db::open_in_memory().unwrap();
    db.call(|conn|{conn.execute("INSERT INTO library(file_path,title,acoustid_id,mb_recording_id,profile_id,added_at,updated_at) VALUES('a.opus','Song','sound','mb','original',0,0)",[])?;assert_eq!(crate::library::find_by_fingerprint(conn,None,Some("mb"))?.len(),1);assert_eq!(crate::library::find_by_fingerprint(conn,Some("sound"),None)?.len(),1);assert!(crate::library::find_by_fingerprint(conn,None,Some("different"))?.is_empty());Ok(())}).await.unwrap();
}

#[tokio::test]
async fn silence_filter_keeps_the_pause_inside_a_track() {
    let dir = tempfile::tempdir().unwrap();
    let path = audio(
        dir.path(),
        "aevalsrc=if(between(t\\,2\\,4)+between(t\\,5\\,8)\\,0.2*sin(2*PI*440*t)\\,0):d=11:s=48000",
    )
    .await;
    let ready = dir.path().join("trim.opus");
    audio::silence(
        &common::ffmpeg(),
        &path,
        &ready,
        &crate::profiles::profile("original")
            .unwrap()
            .resolve("opus"),
        129.0,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    let duration = crate::transcode::probe(&common::ffprobe(), &ready)
        .await
        .unwrap()
        .duration_s;
    assert!(
        (duration - 6.0).abs() < 0.2,
        "Internal pause must stay; got {duration}"
    );
}

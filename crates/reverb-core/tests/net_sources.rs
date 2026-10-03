mod common;
use reverb_core::backend::{DownloadBackend, JobSpec, StaticContext, YtDlpProcessBackend};
use reverb_core::sources::{HttpSource, SourceProvider};
use reverb_core::ytdlp::{YtDlpContext, YtDlpRunner};
use reverb_core::{Db, MemorySink, SettingsService};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

async fn settings() -> Arc<SettingsService> {
    Arc::new(
        SettingsService::new(Db::open_in_memory().unwrap(), Arc::new(MemorySink::new()))
            .await
            .unwrap(),
    )
}

#[tokio::test]
#[ignore = "network"]
async fn f14_archive_real_flac_tags_and_lossless() {
    let db = Db::open_in_memory().unwrap();
    let settings = Arc::new(
        SettingsService::new(db.clone(), Arc::new(MemorySink::new()))
            .await
            .unwrap(),
    );
    let provider = HttpSource::new("archive", settings.clone());
    let cancel = CancellationToken::new();
    let found = provider
        .search("identifier:bach-well-tempered-clavier-book-1", 20, &cancel)
        .await
        .unwrap();
    assert!(found
        .iter()
        .any(|item| item.id == "bach-well-tempered-clavier-book-1"));
    let tracks = provider
        .tracks(
            "https://archive.org/details/bach-well-tempered-clavier-book-1",
            &cancel,
        )
        .await
        .unwrap();
    assert_eq!(tracks.len(), 48);
    assert_eq!(tracks[0].title, "Prelude No. 1 in C major, BWV 846");
    assert_eq!(tracks[0].track_no, Some(1));
    let tmp = tempfile::tempdir().unwrap();
    let ctx = YtDlpContext {
        ytdlp_path: common::tool_dir("ytdlp").join(common::exe("yt-dlp")),
        js_runtime_arg: format!(
            "deno:{}",
            common::tool_dir("deno").join(common::exe("deno")).display()
        ),
        ffmpeg_dir: common::tool_dir("ffmpeg"),
        cookies: None,
        limit_rate_mbps: None,
        pot_args: None,
    };
    let backend: Arc<dyn DownloadBackend> = Arc::new(reverb_core::sources::SourcesBackend::new(
        Arc::new(YtDlpProcessBackend::new(
            YtDlpRunner::new(None),
            Arc::new(StaticContext(ctx)),
        )),
        settings.clone(),
    ));
    let metadata = Arc::new(reverb_core::metadata::MetadataService::new(
        backend.clone(),
        settings.clone(),
        db,
        &reverb_core::metadata::Endpoints::default(),
        reverb_core::metadata::cache::system_clock(),
    ));
    let pipeline = reverb_core::pipeline::DownloadPipeline::new(
        backend,
        tmp.path().join("data"),
        common::tool_dir("ffmpeg"),
        None,
    )
    .with_metadata(Some(metadata))
    .with_postprocessing(Arc::new(
        reverb_core::pipeline::postprocess::PostProcessor::default(),
    ));
    let mut config = settings.get();
    config.fetch_artwork = false;
    config.fetch_lyrics = false;
    config.normalize_volume = false;
    let output = pipeline
        .run(
            &reverb_core::pipeline::PipelineJob {
                job_id: "archive-cc0".into(),
                url: tracks[0].url.clone(),
                profile: reverb_core::profiles::profile("original").unwrap(),
                out_dir: tmp.path().join("music"),
                sponsorblock: None,
                metadata_override: None,
                fetch_metadata: None,
                settings: Some(config),
                options: Default::default(),
                playlist_ctx: None,
            },
            &cancel,
            &|_| {},
        )
        .await
        .unwrap();
    let path = &output.path;
    assert!(
        path.starts_with(
            reverb_core::library::files::canonical(
                &tmp.path().join("music").join("Kimiko Ishizaka")
            )
            .unwrap()
        ),
        "{}",
        path.display()
    );
    assert_eq!(path.extension().unwrap(), "flac");
    assert!(!tmp.path().join("data/tmp/archive-cc0").exists());
    let probe = reverb_core::transcode::probe(&common::ffprobe(), path)
        .await
        .unwrap();
    assert_eq!(probe.codec, "flac");
    let report =
        reverb_core::lossless::verify(&common::ffmpeg(), &common::ffprobe(), path, &cancel)
            .await
            .unwrap();
    eprintln!("Archive {}", report.details);
    assert_eq!(report.verdict.id(), "lossless", "{}", report.details);
    let tags = reverb_core::tagging::read_tags(path).unwrap();
    assert_eq!(tags.title, "Prelude No. 1 in C major, BWV 846");
    assert_eq!(tags.artist.as_deref(), Some("Kimiko Ishizaka"));
    assert_eq!(tags.track_no, Some(1));
    assert_eq!(tags.year, Some(2015));
    assert!(tags.album.as_deref().unwrap().starts_with("Bach: Well"));
}
#[tokio::test]
#[ignore = "network"]
async fn f14_bandcamp_real_free_flac() {
    let provider = HttpSource::new("bandcamp", settings().await);
    let tmp = tempfile::tempdir().unwrap();
    let done = provider
        .download(
            &JobSpec {
                url: "https://soundslikeanearful.bandcamp.com/track/mellow-harmonics".into(),
                tmp_dir: tmp.path().into(),
                sponsorblock: None,
            },
            &|_| {},
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(
        reverb_core::transcode::probe(&common::ffprobe(), std::path::Path::new(&done.filepath))
            .await
            .unwrap()
            .codec,
        "flac"
    );
}
#[tokio::test]
#[ignore = "network"]
async fn f14_soundcloud_real_download() {
    let ctx = YtDlpContext {
        ytdlp_path: common::tool_dir("ytdlp").join(common::exe("yt-dlp")),
        js_runtime_arg: format!(
            "deno:{}",
            common::tool_dir("deno").join(common::exe("deno")).display()
        ),
        ffmpeg_dir: common::tool_dir("ffmpeg"),
        cookies: None,
        limit_rate_mbps: None,
        pot_args: None,
    };
    let backend = YtDlpProcessBackend::new(YtDlpRunner::new(None), Arc::new(StaticContext(ctx)));
    let tmp = tempfile::tempdir().unwrap();
    let url = "https://soundcloud.com/scottbuckley/icarus-cc-by";
    let analysis = backend
        .analyze(url, &CancellationToken::new())
        .await
        .unwrap();
    if let reverb_core::ytdlp::Analysis::Video { info } = analysis {
        eprintln!("SoundCloud formats {:?}", info.audio_formats);
    }
    let done = backend
        .download(
            &JobSpec {
                url: url.into(),
                tmp_dir: tmp.path().into(),
                sponsorblock: None,
            },
            &|_| {},
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    eprintln!("SoundCloud format {:?}", done.format_id);
    assert!(
        reverb_core::transcode::probe(&common::ffprobe(), std::path::Path::new(&done.filepath))
            .await
            .unwrap()
            .duration_s
            > 0.0
    );
}

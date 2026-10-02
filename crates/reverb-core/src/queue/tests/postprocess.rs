//! F09 T7/T8: fila real, Opus sintético e provedores HTTP locais.
use super::*;
use crate::artwork::ArtworkClient;
use crate::lyrics::LyricsClient;
use crate::metadata::{cache::system_clock, provider::http_client, Endpoints, MetadataService};
use crate::pipeline::postprocess::PostProcessor;
use crate::{library, tagging};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

#[path = "../../../tests/common/mod.rs"]
mod common;

const URL: &str = "https://music.youtube.com/watch?v=lYBUbBu4W08";
const LRC: &str = "[00:00.00]<00:00.00>Never <00:01.00>gonna give you up\n";

async fn server(success: bool) -> MockServer {
    let server = MockServer::start().await;
    for (route, fixture) in [
        ("/deezer/search", "deezer-search-fx3.json"),
        ("/deezer/track/14408104", "deezer-track-14408104.json"),
        ("/deezer/album/1321413", "deezer-album-1321413.json"),
        ("/itunes/search", "itunes-search-fx3.json"),
        (
            "/musicbrainz/ws/2/recording",
            "musicbrainz-recording-fx3.json",
        ),
    ] {
        let mut body: Value =
            serde_json::from_str(&metadata::read(&format!("http/{fixture}"))).unwrap();
        fn local_covers(value: &mut Value, url: &str) {
            match value {
                Value::Object(map) => {
                    for (key, item) in map {
                        if (key.starts_with("cover") || key.starts_with("artworkUrl"))
                            && item.is_string()
                        {
                            *item = json!(url);
                        } else {
                            local_covers(item, url);
                        }
                    }
                }
                Value::Array(items) => {
                    for item in items {
                        local_covers(item, url);
                    }
                }
                _ => {}
            }
        }
        local_covers(&mut body, &format!("{}/cover", server.uri()));
        Mock::given(method("GET"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;
    }
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        600,
        600,
        image::Rgb([230, 20, 10]),
    ))
    .write_to(&mut png, image::ImageFormat::Png)
    .unwrap();
    Mock::given(method("GET"))
        .and(path("/cover"))
        .respond_with(if success {
            ResponseTemplate::new(200).set_body_bytes(png.into_inner())
        } else {
            ResponseTemplate::new(500)
        })
        .mount(&server)
        .await;
    Mock::given(method("GET")).and(path("/api/get")).respond_with(if success {
        ResponseTemplate::new(200).set_body_json(json!({ "duration": 10, "plainLyrics": "Never gonna give you up", "syncedLyrics": LRC }))
    } else { ResponseTemplate::new(500) }).mount(&server).await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    server
}

async fn start(env: &Env, server: &MockServer, readonly: bool) -> QueueService {
    let audio = common::pink_noise_opus(env.dir.path()).await;
    env.backend
        .set_audio(std::fs::read(audio).unwrap(), readonly);
    let mut video =
        crate::ytdlp::VideoInfo::from_json(&metadata::read("ytdlp/fx2-music.json")).unwrap();
    video.thumbnail = Some(format!("{}/cover", server.uri()));
    env.backend.set_video(video);
    let service = Arc::new(MetadataService::new(
        env.backend.clone(),
        env.settings.clone(),
        env.db.clone(),
        &Endpoints::all(&server.uri()),
        system_clock(),
    ));
    let mut deps = deps(
        &env.db,
        &env.settings,
        &env.sink,
        &env.backend,
        &env.heal,
        env.dir.path(),
    );
    deps.runner = Arc::new(
        DownloadPipeline::new(
            env.backend.clone(),
            env.dir.path().to_owned(),
            common::tool_dir("ffmpeg"),
            None,
        )
        .with_metadata(Some(service))
        .with_postprocessing(Arc::new(PostProcessor::new(
            ArtworkClient::default(),
            Arc::new(LyricsClient::new(
                http_client(),
                &format!("{}/api", server.uri()),
                LyricsClient::limiter(),
            )),
        ))),
    );
    QueueService::start(deps).await.unwrap()
}

fn request() -> EnqueueRequest {
    EnqueueRequest {
        url: URL.into(),
        source_id: Some("lYBUbBu4W08".into()),
        ..Default::default()
    }
}

async fn finish(queue: &QueueService) -> Job {
    let job = queue.enqueue(request()).await.unwrap();
    metadata::wait_done(queue, &job.id).await
}

fn clean_tmp(env: &Env) {
    assert_eq!(
        std::fs::read_dir(env.dir.path().join("tmp"))
            .unwrap()
            .count(),
        0
    );
}

#[tokio::test]
async fn t7_pipeline_completo_tags_sidecars_biblioteca_fts_e_tmp_limpo() {
    let env = Env::without_service(Fix::Never).await;
    let server = server(true).await;
    let queue = start(&env, &server, false).await;
    let job = finish(&queue).await;
    assert_eq!(job.status, JobStatus::Done, "{job:?}");
    assert!(job.warnings.is_empty(), "{:?}", job.warnings);
    let path = PathBuf::from(job.output_path.as_ref().unwrap());
    assert!(
        path.starts_with(
            env.out_dir()
                .join("Rick Astley")
                .join("Whenever You Need Somebody")
        ),
        "{}",
        path.display()
    );
    let tags = tagging::read_tags(&path).unwrap();
    assert_eq!(tags.title, "Never Gonna Give You Up");
    assert_eq!(tags.artist.as_deref(), Some("Rick Astley"));
    assert_eq!(tags.lyrics.as_deref(), Some(LRC));
    assert!(tags.replay_gain_track_gain.is_some());
    assert!(tags.replay_gain_track_peak.is_some());
    assert!(tags.r128_track_gain.is_some());
    let cover = tags.cover.unwrap();
    let image = image::load_from_memory(&cover.data).unwrap();
    assert_eq!((image.width(), image.height()), (600, 600));
    assert_eq!(
        std::fs::read(path.parent().unwrap().join("cover.jpg")).unwrap(),
        cover.data
    );
    assert_eq!(
        std::fs::read_to_string(path.with_extension("lrc")).unwrap(),
        LRC
    );
    let id = job.library_id.unwrap();
    env.db
        .call(move |conn| {
            let item = library::get(conn, id)?.unwrap();
            assert!(item.cover_source.is_some() && item.has_lyrics && item.has_synced_lyrics);
            assert!(item.replaygain_db.is_some());
            assert_eq!(
                conn.query_row(
                    "SELECT count(*) FROM library_fts WHERE library_fts MATCH 'Never'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            assert_eq!(repo::get(conn, &job.id)?.unwrap().library_id, Some(id));
            Ok(())
        })
        .await
        .unwrap();
    use base64::Engine;
    let thumbnail = library::cover_thumbnail(&path).unwrap().unwrap();
    let jpeg = base64::engine::general_purpose::STANDARD
        .decode(thumbnail.strip_prefix("data:image/jpeg;base64,").unwrap())
        .unwrap();
    let image = image::load_from_memory(&jpeg).unwrap();
    assert_eq!((image.width(), image.height()), (256, 256));
    clean_tmp(&env);
    queue.shutdown().await;
}

#[tokio::test]
async fn t8_falhas_opcionais_concluem_com_avisos_persistidos() {
    let env = Env::without_service(Fix::Never).await;
    let server = server(false).await;
    let queue = start(&env, &server, false).await;
    let job = finish(&queue).await;
    assert_eq!(job.status, JobStatus::Done, "{job:?}");
    assert_eq!(job.warnings, ["warnings.artwork", "warnings.lyrics"]);
    let path = PathBuf::from(job.output_path.unwrap());
    assert!(!path.with_extension("lrc").exists());
    assert!(!path.parent().unwrap().join("cover.jpg").exists());
    assert!(tagging::read_tags(&path)
        .unwrap()
        .replay_gain_track_gain
        .is_some());
    clean_tmp(&env);
    queue.shutdown().await;
}

#[tokio::test]
async fn t8_tag_somente_leitura_falha_com_disk_sem_publicacao() {
    let env = Env::without_service(Fix::Never).await;
    let server = server(true).await;
    let queue = start(&env, &server, true).await;
    let job = finish(&queue).await;
    assert_eq!(job.status, JobStatus::Failed, "{job:?}");
    assert_eq!(job.error_kind.as_deref(), Some("disk"));
    assert!(job.error_message.as_deref().unwrap().contains("leitura"));
    assert!(job.library_id.is_none() && job.output_path.is_none());
    assert_eq!(std::fs::read_dir(env.out_dir()).unwrap().count(), 0);
    clean_tmp(&env);
    queue.shutdown().await;
}

#[tokio::test]
async fn falha_no_commit_da_biblioteca_remove_audio_letra_e_capa() {
    let env = Env::without_service(Fix::Never).await;
    env.db.call(|conn| {
        conn.execute_batch("CREATE TRIGGER reject_library BEFORE INSERT ON library BEGIN SELECT RAISE(ABORT, 'disk test'); END;")?;
        Ok(())
    }).await.unwrap();
    let server = server(true).await;
    let queue = start(&env, &server, false).await;
    let job = finish(&queue).await;
    assert_eq!(job.status, JobStatus::Failed, "{job:?}");
    assert_eq!(job.error_kind.as_deref(), Some("disk"));
    assert!(job.library_id.is_none() && job.output_path.is_none());
    // Espera a saída do runner, que solta o guard de publicação depois de emitir a falha.
    queue.shutdown().await;
    let album = env.out_dir().join("Rick Astley/Whenever You Need Somebody");
    assert_eq!(std::fs::read_dir(album).unwrap().count(), 0);
    env.db
        .call(|conn| {
            assert_eq!(
                conn.query_row("SELECT count(*) FROM library", [], |r| r.get::<_, i64>(0))?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
    clean_tmp(&env);
}

#[tokio::test]
async fn offline_e_opcoes_do_job_pulam_consultas_e_sidecars() {
    let env = Env::without_service(Fix::Never).await;
    env.set(json!({ "offlineMode": true, "normalizeVolume": false }))
        .await;
    let server = server(true).await;
    let queue = start(&env, &server, false).await;
    let mut request = request();
    request.options = Some(JobOptions {
        auto_organize: Some(false),
        ..Default::default()
    });
    let job = queue.enqueue(request).await.unwrap();
    let job = metadata::wait_done(&queue, &job.id).await;
    assert_eq!(job.status, JobStatus::Done, "{job:?}");
    assert!(job.warnings.is_empty());
    let path = PathBuf::from(job.output_path.unwrap());
    assert_eq!(path.parent(), Some(env.out_dir().as_path()));
    let tags = tagging::read_tags(&path).unwrap();
    assert!(tags.lyrics.is_none() && tags.cover.is_none() && tags.replay_gain_track_gain.is_none());
    assert!(server.received_requests().await.unwrap().is_empty());
    clean_tmp(&env);
    queue.shutdown().await;
}

#[tokio::test]
async fn other_nao_consulta_lrclib_e_opcoes_do_job_vencem_settings() {
    let env = Env::without_service(Fix::Never).await;
    let server = server(true).await;
    let queue = start(&env, &server, false).await;
    let mut video =
        crate::ytdlp::VideoInfo::from_json(&metadata::read("ytdlp/fx1-video.json")).unwrap();
    video.thumbnail = Some(format!("{}/cover", server.uri()));
    env.backend.set_video(video);
    let job = queue
        .enqueue(EnqueueRequest {
            url: "https://youtu.be/jNQXAC9IVRw".into(),
            source_id: Some("jNQXAC9IVRw".into()),
            options: Some(JobOptions {
                fetch_artwork: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    let job = metadata::wait_done(&queue, &job.id).await;
    assert_eq!(job.status, JobStatus::Done, "{job:?}");
    assert!(job.warnings.is_empty());
    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "other não precisa de provedores/capa/letra"
    );
    let path = PathBuf::from(job.output_path.unwrap());
    assert_eq!(
        path.strip_prefix(env.out_dir()).unwrap(),
        std::path::Path::new("Outros/jawed/Me at the zoo.opus")
    );
    assert!(!path.with_extension("lrc").exists());
    queue.shutdown().await;
}

#[tokio::test]
async fn raiz_do_job_vence_sync_que_vence_settings() {
    let env = Env::new().await;
    let mut job = env.queue.enqueue(request()).await.unwrap();
    let sync_root = env.dir.path().join("sync");
    let root = sync_root.to_string_lossy().into_owned();
    env.db.call(move |conn| {
        conn.execute("INSERT INTO syncs(id,url,title,profile_id,output_dir,created_at) VALUES('sync','https://youtube.com/playlist?list=teste','Teste','original',?1,0)", [root])?;
        Ok(())
    }).await.unwrap();
    job.sync_id = Some("sync".into());
    assert_eq!(
        super::super::scheduler::output_dir(&env.db, &job, &env.settings.get())
            .await
            .unwrap(),
        sync_root
    );
    let custom = env.dir.path().join("job");
    job.options.output_dir = Some(custom.to_string_lossy().into_owned());
    assert_eq!(
        super::super::scheduler::output_dir(&env.db, &job, &env.settings.get())
            .await
            .unwrap(),
        custom
    );
    job.options.output_dir = Some("  ".into());
    job.sync_id = Some("ausente".into());
    assert_eq!(
        super::super::scheduler::output_dir(&env.db, &job, &env.settings.get())
            .await
            .unwrap(),
        env.out_dir()
    );
    env.queue.shutdown().await;
}

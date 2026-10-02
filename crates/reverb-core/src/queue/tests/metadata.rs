//! F08 — T15 (sem rede) e a fila com os passos `resolve_source` e `identify`: a fonte trocada pela
//! faixa oficial, o resultado gravado no job e o override do usuário.

use std::sync::Arc;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::metadata::cache::system_clock;
use crate::metadata::{Bucket, Endpoints, MetadataService};
use crate::ytdlp::{SearchResult, SearchSource, VideoInfo};

const FX3_URL: &str = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
const OFFICIAL_URL: &str = "https://music.youtube.com/watch?v=lYBUbBu4W08";

fn fixtures() -> String {
    format!("{}/../../tests/fixtures", env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(format!("{}/{relative}", fixtures())).unwrap()
}

async fn providers_server() -> MockServer {
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
        Mock::given(method("GET"))
            .and(path(route))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(read(&format!("http/{fixture}")))
                    .insert_header("content-type", "application/json"),
            )
            .mount(&server)
            .await;
    }
    server
}

fn register(backend: &FakeBackend) {
    for name in [
        "fx2-music.json",
        "fx3-clip.json",
        "analyze-aUajNfZwkjY.json",
        "analyze-rmQuHi7a8Q4.json",
    ] {
        backend.set_video(VideoInfo::from_json(&read(&format!("ytdlp/{name}"))).unwrap());
    }
    backend.set_search(
        SearchSource::YtMusic,
        "Rick Astley Never Gonna Give You Up",
        SearchResult::list_from_json(&read("ytdlp/search-ytmusic-fx3.json")).unwrap(),
    );
}

/// Fila com o `MetadataService` ligado ao pipeline (relógio real: há E/S de rede local).
async fn queue_with_metadata(env: &Env, server: &MockServer) -> QueueService {
    register(&env.backend);
    let service = Arc::new(MetadataService::new(
        env.backend.clone(),
        Arc::clone(&env.settings),
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
            env.dir.path().to_path_buf(),
            PathBuf::from("/nada/ffmpeg"),
            None,
        )
        .with_metadata(Some(service)),
    );
    QueueService::start(deps).await.unwrap()
}

fn fx3_request() -> EnqueueRequest {
    EnqueueRequest {
        url: FX3_URL.to_string(),
        source_id: Some("dQw4w9WgXcQ".to_string()),
        ..EnqueueRequest::default()
    }
}

async fn wait_done(queue: &QueueService, id: &str) -> Job {
    for _ in 0..600 {
        let job = queue.get(id).await.unwrap().unwrap();
        if job.status.is_finished() {
            return job;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("o job {id} não terminou");
}

#[tokio::test]
async fn t15_com_prefer_official_audio_o_job_baixa_a_faixa_oficial() {
    let env = Env::without_service(Fix::Never).await;
    let server = providers_server().await;
    let queue = queue_with_metadata(&env, &server).await;
    let job = queue.enqueue(fx3_request()).await.unwrap();
    let job = wait_done(&queue, &job.id).await;
    assert_eq!(job.status, JobStatus::Done, "{:?}", job.error_message);

    // A fonte do job virou a faixa oficial (não o clipe).
    assert_eq!(job.source_id.as_deref(), Some("lYBUbBu4W08"));
    assert_eq!(job.source_url, OFFICIAL_URL);
    assert_eq!(env.backend.order(), vec![OFFICIAL_URL.to_string()]);

    // E o resultado da identificação ficou gravado no job (e no banco).
    let result = job.metadata_result.as_ref().expect("resultado gravado");
    assert_eq!(result.source, "youtube_music");
    assert_eq!(
        result.fields.album.as_deref(),
        Some("Whenever You Need Somebody")
    );
    assert_eq!(result.fields.year, Some(1987));
    assert_eq!(job.confidence, Some(1.0));
    assert_eq!(job.title.as_deref(), Some("Never Gonna Give You Up"));
    assert_eq!(job.artist.as_deref(), Some("Rick Astley"));
    assert_eq!(result.official.as_ref().unwrap().video_id, "lYBUbBu4W08");
    assert!(
        job.metadata_override.is_none(),
        "o override é só da edição do usuário"
    );

    let stored = env
        .db
        .call({
            let id = job.id.clone();
            move |conn| repo::get(conn, &id)
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.source_id.as_deref(), Some("lYBUbBu4W08"));
    assert_eq!(stored.source_url, OFFICIAL_URL);
    assert_eq!(stored.metadata_result, job.metadata_result);
    assert_eq!(stored.confidence, Some(1.0));
    queue.shutdown().await;
}

#[tokio::test]
async fn t15_com_prefer_official_audio_desligado_a_fonte_permanece() {
    let env = Env::without_service(Fix::Never).await;
    env.set(json!({ "preferOfficialAudio": false })).await;
    let server = providers_server().await;
    let queue = queue_with_metadata(&env, &server).await;
    let job = queue.enqueue(fx3_request()).await.unwrap();
    let job = wait_done(&queue, &job.id).await;
    assert_eq!(job.status, JobStatus::Done, "{:?}", job.error_message);
    assert_eq!(job.source_id.as_deref(), Some("dQw4w9WgXcQ"));
    assert_eq!(job.source_url, FX3_URL);
    assert_eq!(env.backend.order(), vec![FX3_URL.to_string()]);
    let result = job.metadata_result.as_ref().unwrap();
    assert!(result.official.is_none());
    assert_ne!(result.source, "youtube_music");
    assert_eq!(job.title.as_deref(), Some("Never Gonna Give You Up"));
    queue.shutdown().await;
}

#[tokio::test]
async fn override_do_usuario_vai_para_o_job_e_vence_a_identificacao() {
    let env = Env::without_service(Fix::Never).await;
    let server = providers_server().await;
    let queue = queue_with_metadata(&env, &server).await;
    let edit = json!({ "title": "Meu título", "artist": "Meu artista", "album": "Meu álbum" });
    let job = queue
        .enqueue(EnqueueRequest {
            metadata_override: Some(edit.clone()),
            ..fx3_request()
        })
        .await
        .unwrap();
    let job = wait_done(&queue, &job.id).await;
    assert_eq!(job.status, JobStatus::Done, "{:?}", job.error_message);
    assert_eq!(job.metadata_override, Some(edit));
    let result = job.metadata_result.as_ref().unwrap();
    assert_eq!(result.source, "user");
    assert_eq!(result.bucket, Bucket::Auto);
    assert_eq!(result.fields.title, "Meu título");
    assert_eq!(result.fields.album.as_deref(), Some("Meu álbum"));
    assert_eq!(job.title.as_deref(), Some("Meu título"));
    assert_eq!(job.artist.as_deref(), Some("Meu artista"));
    // Sem troca de fonte e sem nenhuma consulta aos provedores.
    assert_eq!(job.source_id.as_deref(), Some("dQw4w9WgXcQ"));
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(env.backend.search_log().is_empty());
    queue.shutdown().await;
}

#[tokio::test]
async fn job_passa_pelos_estagios_analyzing_e_metadata() {
    let env = Env::without_service(Fix::Never).await;
    let server = providers_server().await;
    let queue = queue_with_metadata(&env, &server).await;
    let job = queue.enqueue(fx3_request()).await.unwrap();
    wait_done(&queue, &job.id).await;
    let stages: Vec<String> = env
        .sink
        .events()
        .into_iter()
        .filter(|(name, _)| name == "job://updated")
        .filter_map(|(_, payload)| payload.get("stage")?.as_str().map(str::to_string))
        .collect();
    let first = |stage: &str| stages.iter().position(|s| s == stage);
    let analyzing = first("analyzing").expect("estágio analyzing");
    let downloading = first("downloading").unwrap();
    let metadata = first("metadata").expect("estágio metadata");
    let done = first("done").unwrap();
    assert!(analyzing < metadata && metadata < done, "{stages:?}");
    assert!(downloading < metadata, "{stages:?}");
    queue.shutdown().await;
}

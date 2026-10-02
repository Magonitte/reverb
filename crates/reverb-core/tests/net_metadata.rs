//! F08 — testes de rede (T10d, T13, T14, T15): YouTube Music, Deezer e o pipeline real. Rodam com
//! `npm run verify:net`; usam diretórios temporários e as ferramentas de `.test-tools/`.

mod common;
#[path = "net_metadata/sync.rs"]
mod sync;

use std::sync::Arc;
use std::time::{Duration, Instant};

use reverb_core::backend::{DownloadBackend, ToolsContext, YtDlpProcessBackend};
use reverb_core::metadata::cache::system_clock;
use reverb_core::metadata::deezer::Deezer;
use reverb_core::metadata::provider::http_client;
use reverb_core::metadata::{ContentType, Endpoints, MetadataProvider, MetadataService, Query};
use reverb_core::queue::{
    EnqueueRequest, HealCoordinator, Job, JobStatus, QueueDeps, QueueService, ToolsHeal,
    ToolsPipeline,
};
use reverb_core::ytdlp::{SearchSource, YtDlpRunner};
use reverb_core::{Db, MemorySink, SettingsService, ToolsConfig, ToolsManager};
use tokio_util::sync::CancellationToken;

const FX1: &str = "https://www.youtube.com/watch?v=jNQXAC9IVRw";
const FX2: &str = "https://music.youtube.com/watch?v=lYBUbBu4W08";
const FX3: &str = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
const OFFICIAL_ID: &str = "lYBUbBu4W08";

struct Stack {
    _temp: tempfile::TempDir,
    data: std::path::PathBuf,
    db: Db,
    sink: Arc<MemorySink>,
    settings: Arc<SettingsService>,
    tools: Arc<ToolsManager>,
    backend: Arc<dyn DownloadBackend>,
    metadata: Arc<MetadataService>,
}

async fn stack() -> Stack {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("dados");
    let sink = Arc::new(MemorySink::new());
    let db = Db::open_in_memory().unwrap();
    let settings = Arc::new(
        SettingsService::new(db.clone(), sink.clone())
            .await
            .unwrap(),
    );
    settings
        .update(
            serde_json::from_value(serde_json::json!({ "outputDir": temp.path().join("musicas") }))
                .unwrap(),
        )
        .await
        .unwrap();
    let tools = Arc::new(
        ToolsManager::new(
            ToolsConfig::new(common::tools_root()),
            db.clone(),
            settings.clone(),
            sink.clone(),
        )
        .unwrap(),
    );
    let backend: Arc<dyn DownloadBackend> = Arc::new(YtDlpProcessBackend::new(
        YtDlpRunner::new(Some(tools.clone())),
        Arc::new(ToolsContext::new(tools.clone(), settings.clone())),
    ));
    let metadata = Arc::new(MetadataService::new(
        backend.clone(),
        settings.clone(),
        db.clone(),
        &Endpoints::default(),
        system_clock(),
    ));
    Stack {
        _temp: temp,
        data,
        db,
        sink,
        settings,
        tools,
        backend,
        metadata,
    }
}

// ---------------------------------------------------------------------------------------------
// T10d — E1 na rede
// ---------------------------------------------------------------------------------------------

#[tokio::test]
#[ignore = "network"]
async fn t10d_busca_por_isrc_no_youtube_music_devolve_a_faixa_certa() {
    let stack = stack().await;
    let cancel = CancellationToken::new();
    let original = stack
        .backend
        .search(SearchSource::YtMusic, "GBARL9300135", 3, &cancel)
        .await
        .unwrap();
    assert_eq!(
        original.first().map(|r| r.id.as_str()),
        Some(OFFICIAL_ID),
        "{original:?}"
    );
    // O ISRC da coletânea devolve outra faixa oficial: o ISRC distingue as versões.
    let compilation = stack
        .backend
        .search(SearchSource::YtMusic, "GBARL0600786", 3, &cancel)
        .await
        .unwrap();
    let first = compilation
        .first()
        .expect("resultado para o ISRC da coletânea");
    assert_ne!(first.id, OFFICIAL_ID, "{compilation:?}");
}

fn deezer() -> Deezer {
    Deezer::new(http_client(), "https://api.deezer.com", Deezer::limiter())
}

#[tokio::test]
#[ignore = "network"]
async fn t10d_deezer_por_isrc_e_busca_simples() {
    let by_isrc = deezer()
        .by_isrc("GBARL9300135")
        .await
        .expect("o Deezer conhece o ISRC");
    assert_eq!(by_isrc.title, "Never Gonna Give You Up");
    assert_eq!(by_isrc.isrc.as_deref(), Some("GBARL9300135"));

    let query = Query::new(
        "never gonna give you up",
        vec!["rick astley".to_string()],
        None,
    );
    let found = deezer().search(&query).await;
    let first = found.first().expect("a busca simples devolve resultados");
    assert_eq!(
        first.artists.first().map(String::as_str),
        Some("Rick Astley")
    );
    // O ISRC vem do `/track/{id}` (a busca já o traz, mas os detalhes devem confirmar).
    let detailed = deezer().details(first.clone()).await;
    assert!(
        detailed
            .isrc
            .as_deref()
            .is_some_and(|isrc| !isrc.is_empty()),
        "{detailed:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// T13 / T14 — pré-visualização
// ---------------------------------------------------------------------------------------------

#[tokio::test]
#[ignore = "network"]
async fn t13_preview_do_fx3_traz_a_faixa_oficial() {
    let stack = stack().await;
    let result = stack
        .metadata
        .preview(FX3, None, None, &CancellationToken::new())
        .await
        .unwrap();
    let official = result.official.as_ref().expect("versão oficial");
    assert_eq!(official.video_id, OFFICIAL_ID, "{official:?}");
    assert_eq!(result.fields.artist.as_deref(), Some("Rick Astley"));
    assert_eq!(result.fields.title, "Never Gonna Give You Up");
    assert!(result.confidence >= 0.85, "{}", result.confidence);
    assert_eq!(result.source, "youtube_music");
    assert_eq!(
        result.fields.album.as_deref(),
        Some("Whenever You Need Somebody")
    );
}

#[tokio::test]
#[ignore = "network"]
async fn t14_preview_do_fx1_e_other_sem_candidatos() {
    let stack = stack().await;
    let result = stack
        .metadata
        .preview(FX1, None, None, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(result.content_type, ContentType::Other);
    assert!(result.candidates.is_empty());
    assert!(result.official.is_none());
}

// ---------------------------------------------------------------------------------------------
// T15 — job real
// ---------------------------------------------------------------------------------------------

async fn run_job(stack: &Stack, url: &str, source_id: &str) -> Job {
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
    let job = queue
        .enqueue(EnqueueRequest {
            url: url.to_string(),
            source_id: Some(source_id.to_string()),
            ..EnqueueRequest::default()
        })
        .await
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(300);
    let finished = loop {
        let current = queue.get(&job.id).await.unwrap().unwrap();
        if current.status.is_finished() {
            break current;
        }
        assert!(Instant::now() < deadline, "o job não terminou: {current:?}");
        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    queue.shutdown().await;
    assert_eq!(
        finished.status,
        JobStatus::Done,
        "{:?}",
        finished.error_message
    );
    finished
}

#[tokio::test]
#[ignore = "network"]
async fn t15_job_real_do_fx3_com_prefer_official_audio_baixa_a_faixa_oficial() {
    let stack = stack().await;
    let job = run_job(&stack, FX3, "dQw4w9WgXcQ").await;
    assert_eq!(job.source_id.as_deref(), Some(OFFICIAL_ID), "{job:?}");
    let result = job.metadata_result.as_ref().expect("resultado gravado");
    assert_eq!(result.source, "youtube_music");
    assert_eq!(
        result.fields.album.as_deref(),
        Some("Whenever You Need Somebody")
    );
    assert!(job.confidence.is_some_and(|c| c >= 0.85));
    assert_eq!(job.artist.as_deref(), Some("Rick Astley"));
    assert!(job
        .output_path
        .as_deref()
        .is_some_and(|p| std::path::Path::new(p).is_file()));
}

#[tokio::test]
#[ignore = "network"]
async fn t15_job_real_do_fx3_sem_prefer_official_audio_mantem_o_clipe() {
    let stack = stack().await;
    stack
        .settings
        .update(
            serde_json::from_value(serde_json::json!({ "preferOfficialAudio": false })).unwrap(),
        )
        .await
        .unwrap();
    let job = run_job(&stack, FX3, "dQw4w9WgXcQ").await;
    assert_eq!(job.source_id.as_deref(), Some("dQw4w9WgXcQ"), "{job:?}");
    let result = job.metadata_result.as_ref().expect("resultado gravado");
    assert_ne!(result.source, "youtube_music");
    assert_eq!(job.title.as_deref(), Some("Never Gonna Give You Up"));
    assert_eq!(job.artist.as_deref(), Some("Rick Astley"));
}

#[tokio::test]
#[ignore = "network"]
async fn f09_t11_fx2_tags_capa_letra_replaygain_caminho_e_biblioteca() {
    let stack = stack().await;
    let job = run_job(&stack, FX2, OFFICIAL_ID).await;
    let path = std::path::Path::new(job.output_path.as_ref().unwrap());
    assert!(
        path.starts_with(
            reverb_core::library::files::canonical(
                &stack
                    ._temp
                    .path()
                    .join("musicas/Rick Astley/Whenever You Need Somebody")
            )
            .unwrap()
        ),
        "{}",
        path.display()
    );
    assert!(path
        .to_string_lossy()
        .ends_with("Never Gonna Give You Up.opus"));
    let tags = reverb_core::tagging::read_tags(path).unwrap();
    assert_eq!(tags.title, "Never Gonna Give You Up");
    assert_eq!(tags.artist.as_deref(), Some("Rick Astley"));
    let cover = image::load_from_memory(&tags.cover.unwrap().data).unwrap();
    assert_eq!(cover.width(), cover.height());
    assert!(cover.width() >= 500);
    let lyrics = tags.lyrics.expect("letra embutida");
    assert_eq!(
        std::fs::read_to_string(path.with_extension("lrc")).unwrap(),
        lyrics
    );
    assert!(tags.replay_gain_track_gain.is_some());
    assert!(tags.replay_gain_track_peak.is_some());
    assert!(tags.r128_track_gain.is_some());
    assert!(path.parent().unwrap().join("cover.jpg").is_file());
    let id = job.library_id.expect("biblioteca");
    stack
        .db
        .call(move |conn| {
            let item = reverb_core::library::get(conn, id)?.unwrap();
            assert!(item.has_synced_lyrics && item.replaygain_db.is_some());
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        std::fs::read_dir(stack.data.join("tmp")).unwrap().count(),
        0
    );
}

#[tokio::test]
#[ignore = "network"]
async fn f09_t12_fx1_outros_jawed_sem_letra() {
    let stack = stack().await;
    let job = run_job(&stack, FX1, "jNQXAC9IVRw").await;
    let path = std::path::Path::new(job.output_path.as_ref().unwrap());
    assert_eq!(
        path.strip_prefix(
            reverb_core::library::files::canonical(&stack._temp.path().join("musicas")).unwrap()
        )
        .unwrap(),
        std::path::Path::new("Outros/jawed/Me at the zoo.opus")
    );
    assert!(!path.with_extension("lrc").exists());
    assert!(reverb_core::tagging::read_tags(path)
        .unwrap()
        .lyrics
        .is_none());
    assert!(!job.warnings.iter().any(|w| w == "warnings.lyrics"));
    let id = job.library_id.unwrap();
    stack
        .db
        .call(move |conn| {
            assert_eq!(
                reverb_core::library::get(conn, id)?.unwrap().content_type,
                ContentType::Other
            );
            Ok(())
        })
        .await
        .unwrap();
}

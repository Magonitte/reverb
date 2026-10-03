//! F08 — T8, T9 e T13 (sem rede): o pipeline de metadados com `wiremock`, o `FakeBackend` e as
//! respostas gravadas de FX1, FX2 e FX3.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::db::Db;
use crate::events::MemorySink;
use crate::metadata::cache::system_clock;
use crate::queue::fake::FakeBackend;
use crate::ytdlp::{SearchResult, SearchSource};

const FX2_URL: &str = "https://music.youtube.com/watch?v=lYBUbBu4W08";
const FX3_URL: &str = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
const FX1_URL: &str = "https://www.youtube.com/watch?v=jNQXAC9IVRw";
const SEARCH_FX3: &str = "Rick Astley Never Gonna Give You Up";

fn root() -> String {
    format!("{}/../../tests/fixtures", env!("CARGO_MANIFEST_DIR"))
}

fn ytdlp_fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{}/ytdlp/{name}", root())).unwrap()
}

fn http_fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{}/http/{name}", root())).unwrap()
}

fn video(name: &str) -> VideoInfo {
    VideoInfo::from_json(&ytdlp_fixture(name)).unwrap()
}

fn json_body(name: &str) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_string(http_fixture(name))
        .insert_header("content-type", "application/json")
}

struct Env {
    service: MetadataService,
    server: MockServer,
    backend: Arc<FakeBackend>,
    settings: Arc<SettingsService>,
}

impl Env {
    async fn new() -> Self {
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
                .respond_with(json_body(fixture))
                .mount(&server)
                .await;
        }
        let backend = FakeBackend::new();
        for name in [
            "fx1-video.json",
            "fx2-music.json",
            "fx3-clip.json",
            "analyze-aUajNfZwkjY.json",
            "analyze-rmQuHi7a8Q4.json",
            "analyze-QonqLGRyMLk.json",
            "analyze-0nrIroJpjgg.json",
            "analyze--aIiQj79b6Q.json",
        ] {
            backend.set_video(video(name));
        }
        backend.set_search(
            SearchSource::YtMusic,
            SEARCH_FX3,
            SearchResult::list_from_json(&ytdlp_fixture("search-ytmusic-fx3.json")).unwrap(),
        );
        let db = Db::open_in_memory().unwrap();
        let settings = Arc::new(
            SettingsService::new(db.clone(), Arc::new(MemorySink::new()))
                .await
                .unwrap(),
        );
        let service = MetadataService::new(
            backend.clone(),
            Arc::clone(&settings),
            db,
            &Endpoints::all(&server.uri()),
            system_clock(),
        );
        Self {
            service,
            server,
            backend,
            settings,
        }
    }

    async fn set(&self, patch: serde_json::Value) {
        self.settings
            .update(serde_json::from_value(patch).unwrap())
            .await
            .unwrap();
    }

    async fn requests(&self) -> usize {
        self.server.received_requests().await.unwrap().len()
    }

    async fn identify(
        &self,
        url: &str,
        user_override: Option<&serde_json::Value>,
    ) -> MetadataResult {
        let cancel = CancellationToken::new();
        let plan = self
            .service
            .resolve_source(url, user_override.is_some(), &cancel)
            .await
            .unwrap();
        let duration = plan.video.as_ref().and_then(|v| v.duration);
        let title = plan
            .video
            .as_ref()
            .map(|v| v.title.clone())
            .unwrap_or_default();
        self.service
            .identify(IdentifyInput {
                plan: &plan,
                fallback_title: &title,
                duration_s: duration,
                user_override,
                fetch_metadata: None,
            })
            .await
    }
}

// ---------------------------------------------------------------------------------------------
// T8
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn t8_fx2_oficial_vem_do_youtube_music_com_confianca_1() {
    let env = Env::new().await;
    let result = env.identify(FX2_URL, None).await;
    assert_eq!(result.source, "youtube_music");
    assert_eq!(result.confidence, 1.0);
    assert_eq!(result.bucket, Bucket::Auto);
    assert_eq!(result.content_type, ContentType::Music);
    assert_eq!(result.fields.title, "Never Gonna Give You Up");
    assert_eq!(result.fields.artist.as_deref(), Some("Rick Astley"));
    assert_eq!(
        result.fields.album.as_deref(),
        Some("Whenever You Need Somebody")
    );
    assert_eq!(result.fields.year, Some(1987));
    assert!(result.official.is_none());
    // Já é oficial: nada de busca de versão oficial.
    assert!(env.backend.search_log().is_empty());
}

#[tokio::test]
async fn t8_base_oficial_so_completa_o_que_falta() {
    let env = Env::new().await;
    let result = env.identify(FX2_URL, None).await;
    // O iTunes (pontuação 1,0 e duração mais próxima) completa faixa, total e gênero…
    assert_eq!(result.fields.track_no, Some(1));
    assert_eq!(result.fields.track_total, Some(10));
    assert_eq!(result.fields.genre.as_deref(), Some("Pop"));
    assert!(result.fields.cover_url.is_some());
    // F09 pode usar o complemento aplicado e os demais candidatos sem buscar novamente.
    assert!(!result.candidates.is_empty());
    let covers = crate::artwork::candidates(&result, Some("thumbnail"), 0.85);
    assert_eq!(covers[0].url, result.fields.cover_url.as_deref().unwrap());
    assert_eq!(covers[0].source, "itunes");
    // …mas não troca título, álbum nem ano (que já eram oficiais).
    assert_eq!(
        result.fields.album.as_deref(),
        Some("Whenever You Need Somebody")
    );
    assert_eq!(result.fields.year, Some(1987));
    assert_eq!(result.source, "youtube_music");
}

#[tokio::test]
async fn t8_fx1_nao_e_musica_e_nao_consulta_provedores() {
    let env = Env::new().await;
    let result = env.identify(FX1_URL, None).await;
    assert_eq!(result.content_type, ContentType::Other);
    assert_eq!(result.bucket, Bucket::None);
    assert!(result.candidates.is_empty());
    assert_eq!(result.confidence, 0.0);
    // `other`: título do vídeo e artista = canal.
    assert_eq!(result.fields.title, "Me at the zoo");
    assert_eq!(result.fields.artist.as_deref(), Some("jawed"));
    assert_eq!(result.source, "youtube");
    assert_eq!(env.requests().await, 0, "nenhuma requisição aos provedores");
    assert!(env.backend.search_log().is_empty());
}

#[tokio::test]
async fn t8_modo_offline_so_usa_a_base_e_o_parse() {
    let env = Env::new().await;
    env.set(json!({ "offlineMode": true })).await;
    let result = env.identify(FX3_URL, None).await;
    assert_eq!(env.requests().await, 0);
    assert!(
        env.backend.search_log().is_empty(),
        "nem a busca da versão oficial"
    );
    assert_eq!(result.fields.title, "Never Gonna Give You Up");
    assert_eq!(result.fields.artist.as_deref(), Some("Rick Astley"));
    assert_eq!(result.bucket, Bucket::None);
    assert!(result.official.is_none());
    // O próprio comando também respeita o modo offline.
    let video = video("fx3-clip.json");
    let cancel = CancellationToken::new();
    assert!(env
        .service
        .find_official_version(&video, None, &cancel)
        .await
        .is_none());
    assert!(env
        .service
        .search("rick astley - never gonna give you up")
        .await
        .is_empty());
    assert_eq!(env.requests().await, 0);
}

#[tokio::test]
async fn t8_fetch_metadata_desligado_nao_consulta_provedores() {
    let env = Env::new().await;
    env.set(json!({ "fetchMetadata": false })).await;
    let result = env.identify(FX2_URL, None).await;
    assert_eq!(env.requests().await, 0);
    assert_eq!(result.confidence, 1.0);
    assert_eq!(result.fields.track_no, None);
    // Por job também: `fetch_metadata: Some(false)`.
    env.set(json!({ "fetchMetadata": true })).await;
    let cancel = CancellationToken::new();
    let plan = env
        .service
        .resolve_source(FX2_URL, false, &cancel)
        .await
        .unwrap();
    env.service
        .identify(IdentifyInput {
            plan: &plan,
            fallback_title: "",
            duration_s: Some(214.0),
            user_override: None,
            fetch_metadata: Some(false),
        })
        .await;
    assert_eq!(env.requests().await, 0);
}

/// Provedor falso com candidatos fixos.
struct Fixed(Vec<Candidate>);

#[async_trait]
impl MetadataProvider for Fixed {
    fn id(&self) -> &'static str {
        "deezer"
    }

    async fn search(&self, _query: &Query) -> Vec<Candidate> {
        self.0.clone()
    }
}

fn candidate(title: &str, artist: &str, duration: Option<f64>) -> Candidate {
    let mut candidate = Candidate::new("deezer", title, title);
    candidate.artists = vec![artist.to_string()];
    candidate.duration_s = duration;
    candidate.album = Some("Álbum do candidato".into());
    candidate
}

async fn env_with_fixed(candidates: Vec<Candidate>) -> (MetadataService, Arc<SettingsService>) {
    let backend = FakeBackend::new();
    backend.set_video(video("fx3-clip.json"));
    let db = Db::open_in_memory().unwrap();
    let settings = Arc::new(
        SettingsService::new(db, Arc::new(MemorySink::new()))
            .await
            .unwrap(),
    );
    let service = MetadataService::with_providers(
        backend,
        Arc::clone(&settings),
        vec![Arc::new(Fixed(candidates))],
    );
    (service, settings)
}

async fn identify_clip(service: &MetadataService) -> MetadataResult {
    let cancel = CancellationToken::new();
    // `prefer_official_audio` fica desligado nestes testes: só a identificação importa.
    let plan = service
        .resolve_source(FX3_URL, true, &cancel)
        .await
        .unwrap();
    service
        .identify(IdentifyInput {
            plan: &plan,
            fallback_title: "",
            duration_s: Some(213.0),
            user_override: None,
            fetch_metadata: None,
        })
        .await
}

#[tokio::test]
async fn t8_confianca_intermediaria_vai_para_revisao_com_candidatos() {
    // Título e artista iguais, mas sem duração: a nota fica presa em 0,84 (< 0,85).
    let good = candidate("Never Gonna Give You Up", "Rick Astley", None);
    let worse = candidate("Never Gonna Give You Up (Live)", "Rick Astley", None);
    let (service, _settings) = env_with_fixed(vec![worse.clone(), good.clone()]).await;
    let result = identify_clip(&service).await;
    assert_eq!(result.bucket, Bucket::Review);
    assert!(result.needs_review());
    assert!(
        (0.6..0.85).contains(&result.confidence),
        "{}",
        result.confidence
    );
    assert!((result.confidence - 0.84).abs() < 1e-6);
    assert_eq!(result.candidates.len(), 2);
    assert_eq!(
        result.candidates[0].candidate.title,
        "Never Gonna Give You Up"
    );
    assert!(result.candidates[0].score >= result.candidates[1].score);
    // Mantém a base (título analisado do clipe), sem aplicar o candidato.
    assert_eq!(result.fields.title, "Never Gonna Give You Up");
    assert_eq!(result.fields.album, None);
    assert_eq!(result.source, "youtube");
}

#[tokio::test]
async fn t8_confianca_alta_aplica_o_candidato() {
    let good = candidate("Never Gonna Give You Up", "Rick Astley", Some(214.0));
    let (service, _settings) = env_with_fixed(vec![good]).await;
    let result = identify_clip(&service).await;
    assert_eq!(result.bucket, Bucket::Auto);
    assert!(result.confidence >= 0.85);
    assert_eq!(result.source, "deezer");
    assert_eq!(result.fields.album.as_deref(), Some("Álbum do candidato"));
}

#[tokio::test]
async fn t8_confianca_baixa_mantem_a_base_sem_revisao() {
    let other = candidate("Outra Coisa Totalmente", "Fulano", Some(30.0));
    let (service, _settings) = env_with_fixed(vec![other]).await;
    let result = identify_clip(&service).await;
    assert_eq!(result.bucket, Bucket::None);
    assert!(result.candidates.is_empty());
    assert_eq!(result.confidence, 0.0);
    assert_eq!(result.fields.title, "Never Gonna Give You Up");
    assert_eq!(result.fields.album, None);
}

#[tokio::test]
async fn t8_limiares_vem_das_configuracoes() {
    let good = candidate("Never Gonna Give You Up", "Rick Astley", None);
    let (service, settings) = env_with_fixed(vec![good]).await;
    // Com o limiar de aplicação em 0,8, a nota 0,84 já é aplicada sozinha.
    settings
        .update(serde_json::from_value(json!({ "confidenceAutoApply": 0.8 })).unwrap())
        .await
        .unwrap();
    assert_eq!(identify_clip(&service).await.bucket, Bucket::Auto);
    // E com a revisão acima de 0,84, vira "sem revisão".
    settings
        .update(
            serde_json::from_value(json!({ "confidenceAutoApply": 0.95, "confidenceReview": 0.9 }))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(identify_clip(&service).await.bucket, Bucket::None);
}

// ---------------------------------------------------------------------------------------------
// T9
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn t9_override_do_usuario_vence_tudo_e_pula_os_provedores() {
    let env = Env::new().await;
    let edit = json!({ "title": "Meu título", "artist": "Meu artista", "album": "Meu álbum", "year": 1999 });
    let result = env.identify(FX3_URL, Some(&edit)).await;
    assert_eq!(result.source, "user");
    assert_eq!(result.confidence, 1.0);
    assert_eq!(result.bucket, Bucket::Auto);
    assert_eq!(result.fields.title, "Meu título");
    assert_eq!(result.fields.artist.as_deref(), Some("Meu artista"));
    assert_eq!(result.fields.album.as_deref(), Some("Meu álbum"));
    assert_eq!(result.fields.year, Some(1999));
    assert_eq!(env.requests().await, 0, "pula os provedores");
    assert!(env.backend.search_log().is_empty(), "e a troca de fonte");
    assert!(result.official.is_none());
}

#[tokio::test]
async fn t9_override_so_substitui_o_que_foi_editado() {
    let env = Env::new().await;
    let edit = json!({ "artist": "Só o artista" });
    let result = env.identify(FX2_URL, Some(&edit)).await;
    assert_eq!(result.fields.artist.as_deref(), Some("Só o artista"));
    assert_eq!(result.fields.title, "Never Gonna Give You Up");
    assert_eq!(
        result.fields.album.as_deref(),
        Some("Whenever You Need Somebody")
    );
    assert_eq!(result.source, "user");
}

// ---------------------------------------------------------------------------------------------
// resolve_source e preview (T13 sem rede)
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn resolve_source_troca_o_clipe_pela_faixa_oficial() {
    let env = Env::new().await;
    let cancel = CancellationToken::new();
    let plan = env
        .service
        .resolve_source(FX3_URL, false, &cancel)
        .await
        .unwrap();
    assert!(plan.switched());
    assert_eq!(plan.url, "https://music.youtube.com/watch?v=lYBUbBu4W08");
    assert_eq!(plan.switched_from.as_deref(), Some(FX3_URL));
    assert_eq!(plan.video.as_ref().unwrap().id, "lYBUbBu4W08");
    assert_eq!(plan.official.as_ref().unwrap().video_id, "lYBUbBu4W08");
    assert_eq!(plan.content_type, ContentType::Music);
}

#[tokio::test]
async fn t15_com_prefer_official_audio_desligado_a_fonte_permanece() {
    let env = Env::new().await;
    env.set(json!({ "preferOfficialAudio": false })).await;
    let cancel = CancellationToken::new();
    let plan = env
        .service
        .resolve_source(FX3_URL, false, &cancel)
        .await
        .unwrap();
    assert!(!plan.switched());
    assert_eq!(plan.url, FX3_URL);
    assert_eq!(plan.video.as_ref().unwrap().id, "dQw4w9WgXcQ");
    assert!(plan.official.is_none());
    assert!(env.backend.search_log().is_empty());
}

#[tokio::test]
async fn resolve_source_com_override_nao_troca_a_fonte() {
    let env = Env::new().await;
    let cancel = CancellationToken::new();
    let plan = env
        .service
        .resolve_source(FX3_URL, true, &cancel)
        .await
        .unwrap();
    assert!(!plan.switched());
    assert_eq!(plan.url, FX3_URL);
}

#[tokio::test]
async fn resolve_source_nao_busca_oficial_para_quem_nao_e_musica() {
    let env = Env::new().await;
    let cancel = CancellationToken::new();
    let plan = env
        .service
        .resolve_source(FX1_URL, false, &cancel)
        .await
        .unwrap();
    assert!(!plan.switched());
    assert_eq!(plan.content_type, ContentType::Other);
    assert!(env.backend.search_log().is_empty());
}

#[tokio::test]
async fn resolve_source_segue_com_a_url_original_se_a_analise_falhar() {
    let env = Env::new().await;
    let cancel = CancellationToken::new();
    let plan = env
        .service
        .resolve_source(
            "https://www.youtube.com/watch?v=semAnalise1",
            false,
            &cancel,
        )
        .await
        .unwrap();
    assert!(plan.video.is_none());
    assert!(!plan.switched());
    assert_eq!(plan.url, "https://www.youtube.com/watch?v=semAnalise1");
    // Sem análise, a identificação cai no título que o download informar.
    let result = env
        .service
        .identify(IdentifyInput {
            plan: &plan,
            fallback_title: "Banda - Música (Official Video)",
            duration_s: None,
            user_override: None,
            fetch_metadata: Some(false),
        })
        .await;
    assert_eq!(result.content_type, ContentType::Other);
    assert_eq!(result.fields.title, "Banda - Música (Official Video)");
}

#[tokio::test]
async fn resolve_source_propaga_o_cancelamento() {
    let env = Env::new().await;
    let cancel = CancellationToken::new();
    cancel.cancel();
    let error = env
        .service
        .resolve_source(FX3_URL, false, &cancel)
        .await
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Cancelled);
}

#[tokio::test]
async fn isrc_inferido_pelo_deezer_e_o_plano_b() {
    // Nenhum resultado para o texto; só a busca pelo ISRC do 1º resultado do Deezer.
    let env = Env::new().await;
    env.backend
        .set_search(SearchSource::YtMusic, SEARCH_FX3, Vec::new());
    env.backend.set_search(
        SearchSource::YtMusic,
        "GBARL0600786",
        SearchResult::list_from_json(&ytdlp_fixture("search-ytmusic-isrc-GBARL0600786.json"))
            .unwrap(),
    );
    let cancel = CancellationToken::new();
    let plan = env
        .service
        .resolve_source(FX3_URL, false, &cancel)
        .await
        .unwrap();
    assert_eq!(plan.official.as_ref().unwrap().video_id, "-aIiQj79b6Q");
    assert_eq!(
        plan.official.as_ref().unwrap().isrc.as_deref(),
        Some("GBARL0600786")
    );
    let log = env.backend.search_log();
    assert_eq!(
        log[0],
        format!("ytmusic:{SEARCH_FX3}"),
        "o texto vem primeiro"
    );
    assert!(log.contains(&"ytmusic:GBARL0600786".to_string()));
}

#[tokio::test]
async fn t13_preview_do_fx3_traz_a_faixa_oficial() {
    let env = Env::new().await;
    let cancel = CancellationToken::new();
    let result = env
        .service
        .preview(FX3_URL, None, None, &cancel)
        .await
        .unwrap();
    let official = result.official.as_ref().expect("versão oficial sugerida");
    assert_eq!(official.video_id, "lYBUbBu4W08");
    assert!(official.score >= 0.80);
    assert_eq!(result.fields.title, "Never Gonna Give You Up");
    assert_eq!(result.fields.artist.as_deref(), Some("Rick Astley"));
    assert_eq!(
        result.fields.album.as_deref(),
        Some("Whenever You Need Somebody")
    );
    assert_eq!(result.source, "youtube_music");
    assert!(result.confidence >= 0.85);
    assert_eq!(result.content_type, ContentType::Music);
}

#[tokio::test]
async fn preview_com_a_oficial_desmarcada_mostra_os_dados_do_clipe_mas_ainda_sugere() {
    let env = Env::new().await;
    let cancel = CancellationToken::new();
    let result = env
        .service
        .preview(FX3_URL, Some(video("fx3-clip.json")), Some(false), &cancel)
        .await
        .unwrap();
    assert_eq!(result.official.as_ref().unwrap().video_id, "lYBUbBu4W08");
    assert_ne!(result.source, "youtube_music");
    assert_eq!(result.fields.title, "Never Gonna Give You Up");
    assert_eq!(result.fields.artist.as_deref(), Some("Rick Astley"));
}

#[tokio::test]
async fn t14_preview_do_fx1_e_other_sem_candidatos() {
    let env = Env::new().await;
    let cancel = CancellationToken::new();
    let result = env
        .service
        .preview(FX1_URL, None, None, &cancel)
        .await
        .unwrap();
    assert_eq!(result.content_type, ContentType::Other);
    assert!(result.candidates.is_empty());
    assert!(result.official.is_none());
    assert_eq!(env.requests().await, 0);
}

#[tokio::test]
async fn preview_de_colecao_e_recusado() {
    let env = Env::new().await;
    let cancel = CancellationToken::new();
    // O fake não tem análise para esta URL ⇒ erro do backend, repassado.
    assert!(env
        .service
        .preview(
            "https://www.youtube.com/playlist?list=PLx",
            None,
            None,
            &cancel
        )
        .await
        .is_err());
}

#[tokio::test]
async fn busca_manual_junta_os_provedores_com_o_melhor_primeiro() {
    let env = Env::new().await;
    let found = env
        .service
        .search("Rick Astley - Never Gonna Give You Up")
        .await;
    assert!(!found.is_empty());
    assert!(found.len() <= 15);
    let providers: std::collections::BTreeSet<&str> =
        found.iter().map(|c| c.provider.as_str()).collect();
    assert!(providers.contains("deezer") && providers.contains("itunes"));
    assert_eq!(found[0].title, "Never Gonna Give You Up");
    assert!(env.service.search("   ").await.is_empty());
}

#[tokio::test]
async fn full_album_with_chapters_keeps_its_source_and_timeline() {
    let env = Env::new().await;
    let mut raw: serde_json::Value = serde_json::from_str(&ytdlp_fixture("fx3-clip.json")).unwrap();
    raw["duration"] = json!(601);
    raw["chapters"] = json!([{"title":"One","start_time":0,"end_time":300},{"title":"Two","start_time":300,"end_time":601}]);
    env.backend
        .set_video(VideoInfo::from_json(&raw.to_string()).unwrap());
    let plan = env
        .service
        .resolve_source(FX3_URL, false, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(plan.url, FX3_URL);
    assert!(!plan.switched());
    assert_eq!(plan.video.unwrap().chapters.len(), 2);
    assert!(env.backend.search_log().is_empty());
}

//! F08 — T5, T6, T7 e T10c: provedores com `wiremock` e as respostas HTTP gravadas.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::cache::{Cache, CachedProvider, TTL_SECS};
use super::deezer::Deezer;
use super::itunes::Itunes;
use super::musicbrainz::MusicBrainz;
use super::provider::{
    http_client, user_agent, Candidate, Endpoints, MetadataProvider, Query, RateLimiter,
};
use crate::db::Db;

fn http_fixture(name: &str) -> String {
    let path = format!(
        "{}/../../tests/fixtures/http/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

fn json_body(name: &str) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_string(http_fixture(name))
        .insert_header("content-type", "application/json")
}

fn rick() -> Query {
    Query::new(
        "Never Gonna Give You Up",
        vec!["Rick Astley".to_string()],
        Some(213.0),
    )
}

fn deezer(server: &MockServer) -> Deezer {
    Deezer::new(
        http_client(),
        &Endpoints::all(&server.uri()).deezer,
        RateLimiter::new(Duration::ZERO),
    )
}

fn itunes(server: &MockServer) -> Itunes {
    Itunes::new(
        http_client(),
        &Endpoints::all(&server.uri()).itunes,
        RateLimiter::new(Duration::ZERO),
    )
}

fn musicbrainz(server: &MockServer) -> MusicBrainz {
    let endpoints = Endpoints::all(&server.uri());
    MusicBrainz::new(
        http_client(),
        &endpoints.musicbrainz,
        &endpoints.cover_art,
        RateLimiter::new(Duration::ZERO),
    )
}

// ---------------------------------------------------------------------------------------------
// T5 — parsing de cada provedor
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn t5_deezer_interpreta_a_busca_gravada() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/deezer/search"))
        .respond_with(json_body("deezer-search-fx3.json"))
        .mount(&server)
        .await;
    let found = deezer(&server).search(&rick()).await;
    assert_eq!(found.len(), 5);
    let first = &found[0];
    assert_eq!(first.provider, "deezer");
    assert_eq!(first.provider_id, "14408104");
    assert_eq!(first.title, "Never Gonna Give You Up");
    assert_eq!(first.artists, vec!["Rick Astley"]);
    assert_eq!(first.album.as_deref(), Some("Reeling In The Decades"));
    assert_eq!(first.duration_s, Some(211.0));
    assert_eq!(first.isrc.as_deref(), Some("GBARL0600786"));
    assert!(first
        .cover_url
        .as_deref()
        .is_some_and(|u| u.starts_with("https://")));
    assert!(found.iter().all(|c| c.artists == vec!["Rick Astley"]));
}

#[tokio::test]
async fn t5_deezer_detalhes_trazem_faixa_genero_e_total() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/deezer/track/14408104"))
        .respond_with(json_body("deezer-track-14408104.json"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/deezer/album/1321413"))
        .respond_with(json_body("deezer-album-1321413.json"))
        .mount(&server)
        .await;
    let mut candidate = Candidate::new("deezer", "14408104", "Never Gonna Give You Up");
    candidate.artists = vec!["Rick Astley".into()];
    let detailed = deezer(&server).details(candidate).await;
    assert_eq!(detailed.isrc.as_deref(), Some("GBARL0600786"));
    assert_eq!(detailed.track_no, Some(12));
    assert_eq!(detailed.disc_no, Some(2));
    assert_eq!(detailed.year, Some(2011));
    assert_eq!(detailed.genre.as_deref(), Some("Pop"));
    assert_eq!(detailed.track_total, Some(42));
}

#[tokio::test]
async fn t5_deezer_busca_reversa_por_isrc() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/deezer/track/isrc:GBARL9300135"))
        .respond_with(json_body("deezer-isrc-GBARL9300135.json"))
        .mount(&server)
        .await;
    let found = deezer(&server).by_isrc("GBARL9300135").await.unwrap();
    assert_eq!(found.title, "Never Gonna Give You Up");
    assert_eq!(found.isrc.as_deref(), Some("GBARL9300135"));
    assert_eq!(found.artists, vec!["Rick Astley"]);
}

#[tokio::test]
async fn t5_deezer_erro_no_corpo_vira_lista_vazia() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/deezer/search"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"error":{"type":"DataException","message":"no data","code":800}}"#,
        ))
        .mount(&server)
        .await;
    assert!(deezer(&server).search(&rick()).await.is_empty());
}

#[tokio::test]
async fn t5_itunes_interpreta_a_busca_gravada() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/itunes/search"))
        .and(query_param("entity", "song"))
        .and(query_param("limit", "5"))
        .respond_with(
            // O iTunes responde com `text/javascript`.
            ResponseTemplate::new(200)
                .set_body_string(http_fixture("itunes-search-fx3.json"))
                .insert_header("content-type", "text/javascript; charset=utf-8"),
        )
        .mount(&server)
        .await;
    let found = itunes(&server).search(&rick()).await;
    assert_eq!(found.len(), 5);
    let first = &found[0];
    assert_eq!(first.provider, "itunes");
    assert_eq!(first.provider_id, "1559885421");
    assert_eq!(first.title, "Never Gonna Give You Up");
    assert_eq!(first.artists, vec!["Rick Astley"]);
    assert_eq!(first.album.as_deref(), Some("Whenever You Need Somebody"));
    assert_eq!(first.track_no, Some(1));
    assert_eq!(first.track_total, Some(10));
    assert_eq!(first.disc_no, Some(1));
    assert_eq!(first.genre.as_deref(), Some("Pop"));
    assert!((first.duration_s.unwrap() - 213.573).abs() < 1e-6);
    assert_eq!(first.year, Some(1987));
    let cover = first.cover_url.as_deref().unwrap();
    assert!(cover.contains("1200x1200bb"), "{cover}");
    assert!(!cover.contains("100x100bb"));
}

#[tokio::test]
async fn t5_musicbrainz_interpreta_a_busca_gravada() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/musicbrainz/ws/2/recording"))
        .and(query_param("fmt", "json"))
        .and(query_param("limit", "5"))
        .respond_with(json_body("musicbrainz-recording-fx3.json"))
        .mount(&server)
        .await;
    let found = musicbrainz(&server).search(&rick()).await;
    assert_eq!(found.len(), 5);
    let first = &found[0];
    assert_eq!(first.provider, "musicbrainz");
    assert_eq!(first.title, "Never Gonna Give You Up");
    assert_eq!(first.artists, vec!["Rick Astley"]);
    assert_eq!(
        first.mb_recording_id.as_deref(),
        Some(first.provider_id.as_str())
    );
    assert_eq!(first.duration_s, Some(91.0));
    assert!(found.iter().any(|c| c.album.is_some() && c.year.is_some()));
    let with_cover = found.iter().find_map(|c| c.cover_url.as_deref()).unwrap();
    assert!(
        with_cover.starts_with(&format!("{}/coverart/release/", server.uri())),
        "{with_cover}"
    );
    assert!(with_cover.ends_with("/front-1200"));
    // A consulta é Lucene com a gravação e o artista.
    let requests = server.received_requests().await.unwrap();
    let query = requests[0].url.query().unwrap().to_string();
    let decoded = url::form_urlencoded::parse(query.as_bytes())
        .find(|(k, _)| k == "query")
        .map(|(_, v)| v.into_owned())
        .unwrap();
    assert_eq!(
        decoded,
        "recording:\"Never Gonna Give You Up\" AND artist:\"Rick Astley\""
    );
}

#[tokio::test]
async fn t5_respostas_sem_resultado_dao_lista_vazia() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/deezer/search"))
        .respond_with(json_body("deezer-search-fx1.json"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/musicbrainz/ws/2/recording"))
        .respond_with(json_body("musicbrainz-recording-fx1.json"))
        .mount(&server)
        .await;
    let zoo = Query::new("Me at the zoo", vec!["jawed".into()], Some(19.0));
    assert!(deezer(&server).search(&zoo).await.is_empty());
    assert!(musicbrainz(&server).search(&zoo).await.is_empty());
}

#[tokio::test]
async fn t5_erro_http_500_vira_lista_vazia_sem_derrubar() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    assert!(deezer(&server).search(&rick()).await.is_empty());
    assert!(itunes(&server).search(&rick()).await.is_empty());
    assert!(musicbrainz(&server).search(&rick()).await.is_empty());
    let detailed = deezer(&server)
        .details(Candidate::new("deezer", "1", "x"))
        .await;
    assert_eq!(detailed, Candidate::new("deezer", "1", "x"));
}

#[tokio::test]
async fn t5_json_invalido_vira_lista_vazia() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>nope</html>"))
        .mount(&server)
        .await;
    assert!(deezer(&server).search(&rick()).await.is_empty());
    assert!(itunes(&server).search(&rick()).await.is_empty());
}

#[tokio::test]
async fn t5_timeout_vira_lista_vazia() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(http_fixture("deezer-search-fx3.json"))
                .set_delay(Duration::from_millis(1500)),
        )
        .mount(&server)
        .await;
    let impatient = reqwest::Client::builder()
        .timeout(Duration::from_millis(150))
        .build()
        .unwrap();
    let provider = Deezer::new(
        impatient,
        &Endpoints::all(&server.uri()).deezer,
        RateLimiter::new(Duration::ZERO),
    );
    assert!(provider.search(&rick()).await.is_empty());
}

#[tokio::test]
async fn requisicoes_levam_user_agent_identificavel() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(json_body("deezer-search-fx3.json"))
        .mount(&server)
        .await;
    deezer(&server).search(&rick()).await;
    let requests = server.received_requests().await.unwrap();
    let agent = requests[0]
        .headers
        .get("user-agent")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(agent, user_agent());
    assert!(agent.starts_with("Reverb/"));
    assert!(agent.contains("github.com"));
}

// ---------------------------------------------------------------------------------------------
// T10c — busca simples do Deezer
// ---------------------------------------------------------------------------------------------

/// Responde como o Deezer real: a forma avançada combinada (`artist:"A" track:"T"`) dá 0 resultados.
struct RejectsCombinedAdvancedQuery;

impl wiremock::Respond for RejectsCombinedAdvancedQuery {
    fn respond(&self, request: &wiremock::Request) -> ResponseTemplate {
        let q = request
            .url
            .query_pairs()
            .find(|(k, _)| k == "q")
            .map(|(_, v)| v.into_owned())
            .unwrap_or_default();
        if q.contains("artist:\"") && q.contains("track:\"") {
            return ResponseTemplate::new(200).set_body_string(r#"{"data":[],"total":0}"#);
        }
        json_body("deezer-search-fx3.json")
    }
}

#[tokio::test]
async fn t10c_deezer_usa_a_busca_simples() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/deezer/search"))
        .respond_with(RejectsCombinedAdvancedQuery)
        .mount(&server)
        .await;
    let found = deezer(&server).search(&rick()).await;
    assert!(
        !found.is_empty(),
        "a consulta avançada combinada voltou vazia"
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let q = requests[0]
        .url
        .query_pairs()
        .find(|(k, _)| k == "q")
        .map(|(_, v)| v.into_owned())
        .unwrap();
    assert_eq!(q, "Rick Astley Never Gonna Give You Up");
    assert!(!(q.contains("artist:\"") && q.contains("track:\"")));
}

// ---------------------------------------------------------------------------------------------
// T6 — limite de taxa
// ---------------------------------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn t6_musicbrainz_cinco_consultas_levam_ao_menos_4_s() {
    let limiter = MusicBrainz::limiter();
    let start = tokio::time::Instant::now();
    for _ in 0..5 {
        limiter.acquire().await;
    }
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_secs(4), "{elapsed:?}");
    assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
}

#[tokio::test(start_paused = true)]
async fn t6_limitadores_dos_outros_provedores() {
    let deezer = Deezer::limiter();
    let start = tokio::time::Instant::now();
    for _ in 0..6 {
        deezer.acquire().await;
    }
    // 5 por segundo ⇒ 6 chamadas cabem em 1 s.
    assert!(start.elapsed() >= Duration::from_millis(1000));
    assert!(start.elapsed() < Duration::from_millis(1100));
    let itunes = Itunes::limiter();
    let start = tokio::time::Instant::now();
    for _ in 0..4 {
        itunes.acquire().await;
    }
    assert!(start.elapsed() >= Duration::from_millis(990));
}

#[tokio::test(start_paused = true)]
async fn t6_o_limitador_e_compartilhado_entre_tarefas() {
    let limiter = MusicBrainz::limiter();
    let start = tokio::time::Instant::now();
    let tasks: Vec<_> = (0..3)
        .map(|_| {
            let limiter = Arc::clone(&limiter);
            tokio::spawn(async move { limiter.acquire().await })
        })
        .collect();
    for task in tasks {
        task.await.unwrap();
    }
    assert!(start.elapsed() >= Duration::from_secs(2));
}

// ---------------------------------------------------------------------------------------------
// T7 — cache
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn t7_segunda_consulta_igual_nao_chama_o_servidor_e_expirada_chama() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/deezer/search"))
        .respond_with(json_body("deezer-search-fx3.json"))
        .mount(&server)
        .await;
    let now = Arc::new(AtomicI64::new(1_800_000_000));
    let clock = {
        let now = Arc::clone(&now);
        Arc::new(move || now.load(Ordering::SeqCst))
    };
    let cache = Cache::new(Db::open_in_memory().unwrap(), clock);
    let provider = CachedProvider::new(Arc::new(deezer(&server)), cache);

    let first = provider.search(&rick()).await;
    assert_eq!(first.len(), 5);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);

    // Igual (mesma duração, outra grafia do título): vem do cache.
    let same = Query::new(
        "never gonna give you up (Official Video)",
        vec!["RICK ASTLEY".to_string()],
        Some(213.4),
    );
    let second = provider.search(&same).await;
    assert_eq!(second, first);
    assert_eq!(server.received_requests().await.unwrap().len(), 1);

    // Duração diferente (arredondada) ⇒ outra chave.
    let other = Query::new(
        "Never Gonna Give You Up",
        vec!["Rick Astley".to_string()],
        Some(250.0),
    );
    provider.search(&other).await;
    assert_eq!(server.received_requests().await.unwrap().len(), 2);

    // No limite de 30 dias ainda vale.
    now.fetch_add(TTL_SECS, Ordering::SeqCst);
    provider.search(&rick()).await;
    assert_eq!(server.received_requests().await.unwrap().len(), 2);

    // Passou de 30 dias ⇒ consulta de novo.
    now.fetch_add(1, Ordering::SeqCst);
    let refreshed = provider.search(&rick()).await;
    assert_eq!(refreshed.len(), 5);
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
    // E o cache foi renovado.
    provider.search(&rick()).await;
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[tokio::test]
async fn t7_falhas_nao_ficam_no_cache() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let cache = Cache::new(Db::open_in_memory().unwrap(), super::cache::system_clock());
    let provider = CachedProvider::new(Arc::new(deezer(&server)), cache);
    assert!(provider.search(&rick()).await.is_empty());
    assert!(provider.search(&rick()).await.is_empty());
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn t7_detalhes_tambem_usam_o_cache() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/deezer/track/14408104"))
        .respond_with(json_body("deezer-track-14408104.json"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/deezer/album/1321413"))
        .respond_with(json_body("deezer-album-1321413.json"))
        .mount(&server)
        .await;
    let cache = Cache::new(Db::open_in_memory().unwrap(), super::cache::system_clock());
    let provider = CachedProvider::new(Arc::new(deezer(&server)), cache);
    let candidate = Candidate::new("deezer", "14408104", "Never Gonna Give You Up");
    let first = provider.details(candidate.clone()).await;
    let second = provider.details(candidate).await;
    assert_eq!(first, second);
    assert_eq!(first.genre.as_deref(), Some("Pop"));
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

use std::time::Duration;

use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::tools::testutil::fixture_text;

fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("Reverb-teste")
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap()
}

fn release_body(tag: &str) -> serde_json::Value {
    json!({
        "tag_name": tag,
        "zipball_url": "https://example.test/zip",
        "assets": [{
            "name": "yt-dlp.exe",
            "browser_download_url": "https://example.test/yt-dlp.exe",
            "updated_at": "2026-09-30T19:00:51Z",
            "size": 10
        }]
    })
}

async fn serve(server: &MockServer, tag: &str) {
    Mock::given(method("GET"))
        .and(path("/repos/yt-dlp/yt-dlp/releases/latest"))
        .and(header("accept", "application/vnd.github+json"))
        .and(header("user-agent", "Reverb-teste"))
        .respond_with(ResponseTemplate::new(200).set_body_json(release_body(tag)))
        .mount(server)
        .await;
}

#[tokio::test]
async fn envia_user_agent_e_accept_e_le_o_release() {
    let server = MockServer::start().await;
    serve(&server, "2026.10.01").await;
    let client = GithubClient::new(http(), server.uri(), None, None);

    let release = client.latest("yt-dlp/yt-dlp", false).await.unwrap();
    assert_eq!(release.tag_name, "2026.10.01");
    assert_eq!(release.assets.len(), 1);
    assert_eq!(
        release.asset("yt-dlp.exe").unwrap().updated_at.as_deref(),
        Some("2026-09-30T19:00:51Z")
    );
}

#[tokio::test]
async fn envia_o_token_quando_existe() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/releases/latest"))
        .and(header("authorization", "Bearer segredo-de-teste"))
        .respond_with(ResponseTemplate::new(200).set_body_json(release_body("v1")))
        .mount(&server)
        .await;
    let com_token = GithubClient::new(http(), server.uri(), Some("segredo-de-teste".into()), None);
    assert_eq!(com_token.latest("o/r", false).await.unwrap().tag_name, "v1");

    let sem_token = GithubClient::new(http(), server.uri(), None, None);
    assert_eq!(
        sem_token.latest("o/r", false).await.unwrap_err().kind(),
        "github_http"
    );
    // Token vazio é o mesmo que ausente.
    let vazio = GithubClient::new(http(), server.uri(), Some(String::new()), None);
    assert!(vazio.latest("o/r", false).await.is_err());
}

#[tokio::test]
async fn cache_de_uma_hora_e_force() {
    let server = MockServer::start().await;
    serve(&server, "2026.10.01").await;
    let client = GithubClient::new(http(), server.uri(), None, None);

    client.latest("yt-dlp/yt-dlp", false).await.unwrap();
    client.latest("yt-dlp/yt-dlp", false).await.unwrap();
    assert_eq!(server.received_requests().await.unwrap().len(), 1);

    client.latest("yt-dlp/yt-dlp", true).await.unwrap();
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn cache_vencido_busca_de_novo() {
    let server = MockServer::start().await;
    serve(&server, "2026.10.01").await;
    let db = Db::open_in_memory().unwrap();
    // Entrada gravada há mais de 1 h.
    let stale = CachedRelease {
        fetched_at: now_secs() - CACHE_TTL_SECS - 5,
        release: serde_json::from_value(release_body("2026.01.01")).unwrap(),
    };
    db.kv_set(
        "github.latest.yt-dlp/yt-dlp",
        &serde_json::to_string(&stale).unwrap(),
    )
    .await
    .unwrap();

    let client = GithubClient::new(http(), server.uri(), None, Some(db));
    let release = client.latest("yt-dlp/yt-dlp", false).await.unwrap();
    assert_eq!(release.tag_name, "2026.10.01");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn cache_persiste_no_kv_entre_clientes() {
    let server = MockServer::start().await;
    serve(&server, "2026.10.01").await;
    let db = Db::open_in_memory().unwrap();

    let first = GithubClient::new(http(), server.uri(), None, Some(db.clone()));
    first.latest("yt-dlp/yt-dlp", false).await.unwrap();

    // "Reinício do app": cliente novo, mesmo banco ⇒ não consulta a API.
    let second = GithubClient::new(http(), server.uri(), None, Some(db));
    let release = second.latest("yt-dlp/yt-dlp", false).await.unwrap();
    assert_eq!(release.tag_name, "2026.10.01");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn limite_de_requisicoes_e_erros_http() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/limitado/releases/latest"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/ausente/releases/latest"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/lixo/releases/latest"))
        .respond_with(ResponseTemplate::new(200).set_body_string("não é json"))
        .mount(&server)
        .await;
    let client = GithubClient::new(http(), server.uri(), None, None);

    assert_eq!(
        client.latest("o/limitado", false).await.unwrap_err().kind(),
        "github_rate_limit"
    );
    assert_eq!(
        client.latest("o/ausente", false).await.unwrap_err().kind(),
        "github_http"
    );
    assert_eq!(
        client.latest("o/lixo", false).await.unwrap_err().kind(),
        "github_parse"
    );
}

#[test]
fn interpreta_as_respostas_reais() {
    for name in [
        "ytdlp-stable",
        "ytdlp-nightly",
        "deno",
        "ffmpeg",
        "fpcalc",
        "bgutil",
    ] {
        let release: Release =
            serde_json::from_str(&fixture_text(&format!("github/{name}.release.json"))).unwrap();
        assert!(!release.tag_name.is_empty(), "{name}");
        assert!(!release.assets.is_empty(), "{name}");
        assert!(
            release.assets.iter().all(|a| a.updated_at.is_some()),
            "{name}"
        );
    }
    let ffmpeg: Release =
        serde_json::from_str(&fixture_text("github/ffmpeg.release.json")).unwrap();
    // Tag rolante: não dá para comparar por tag (armadilha da F02).
    assert_eq!(ffmpeg.tag_name, "latest");
}

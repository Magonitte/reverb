use super::*;
use serde_json::{json, Value};
use wiremock::{
    matchers::{method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

const ENHANCED: &str = " [00:12.34]<00:12.34>Uma <00:12.90>palavra\r\n[00:15.00]Outra linha\r\n";

fn query() -> LyricsQuery {
    LyricsQuery {
        artist: "Artista & Convidado".into(),
        title: "Canção / Título".into(),
        album: Some("Álbum".into()),
        duration_s: 214.0,
    }
}

fn record(duration: f64, plain: Option<&str>, synced: Option<&str>) -> Value {
    json!({ "duration": duration, "plainLyrics": plain, "syncedLyrics": synced })
}

fn client(server: &MockServer) -> LyricsClient {
    LyricsClient::new(
        http_client(),
        &format!("{}/api/", server.uri()),
        LyricsClient::limiter(),
    )
}

async fn get(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("GET"))
        .and(path("/api/get"))
        .and(query_param("artist_name", "Artista & Convidado"))
        .and(query_param("track_name", "Canção / Título"))
        .and(query_param("album_name", "Álbum"))
        .and(query_param("duration", "214"))
        .respond_with(response)
        .expect(1)
        .mount(server)
        .await;
}

async fn search(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("artist_name", "Artista & Convidado"))
        .and(query_param("track_name", "Canção / Título"))
        .respond_with(response)
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn t2_get_sincronizada_preserva_enhanced_lrc_byte_a_byte() {
    let server = MockServer::start().await;
    get(
        &server,
        ResponseTemplate::new(200).set_body_json(record(214.0, Some("Simples"), Some(ENHANCED))),
    )
    .await;
    let lyrics = client(&server).fetch(&query()).await.unwrap().unwrap();
    assert_eq!(lyrics.plain.as_deref(), Some("Simples"));
    assert_eq!(lyrics.synced.unwrap().as_bytes(), ENHANCED.as_bytes());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn t2_get_so_simples_no_limite_de_2_segundos() {
    let server = MockServer::start().await;
    get(
        &server,
        ResponseTemplate::new(200).set_body_json(record(216.0, Some("  Letra\n"), None)),
    )
    .await;
    assert_eq!(
        client(&server).fetch(&query()).await.unwrap(),
        Some(Lyrics {
            plain: Some("  Letra\n".into()),
            synced: None,
        })
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn t2_404_search_escolhe_menor_delta_com_letra() {
    let server = MockServer::start().await;
    get(&server, ResponseTemplate::new(404)).await;
    search(
        &server,
        ResponseTemplate::new(200).set_body_json(json!([
            record(216.0, None, Some("[00:00.00]Mais distante")),
            record(214.0, None, None),
            record(213.5, Some("Mais próxima"), Some(ENHANCED)),
            record(214.8, Some("Outra"), None),
        ])),
    )
    .await;
    let lyrics = client(&server).fetch(&query()).await.unwrap().unwrap();
    assert_eq!(lyrics.plain.as_deref(), Some("Mais próxima"));
    assert_eq!(lyrics.synced.as_deref(), Some(ENHANCED));
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests[1].url.query_pairs().count(), 2);
}

#[tokio::test]
async fn t2_nada_retorna_none() {
    let server = MockServer::start().await;
    get(&server, ResponseTemplate::new(404)).await;
    search(&server, ResponseTemplate::new(200).set_body_json(json!([]))).await;
    assert!(client(&server).fetch(&query()).await.unwrap().is_none());
}

#[tokio::test]
async fn get_duracao_fora_do_limite_tenta_search_aceita_3_segundos() {
    let server = MockServer::start().await;
    get(
        &server,
        ResponseTemplate::new(200).set_body_json(record(216.01, Some("Get"), None)),
    )
    .await;
    search(
        &server,
        ResponseTemplate::new(200).set_body_json(json!([
            record(217.01, Some("Fora"), None),
            record(211.0, Some("No limite"), None),
        ])),
    )
    .await;
    assert_eq!(
        client(&server)
            .fetch(&query())
            .await
            .unwrap()
            .unwrap()
            .plain
            .as_deref(),
        Some("No limite")
    );
}

#[tokio::test]
async fn descarta_duracao_distante_e_letras_vazias() {
    let server = MockServer::start().await;
    get(
        &server,
        ResponseTemplate::new(200).set_body_json(record(214.0, Some("  \n"), None)),
    )
    .await;
    search(
        &server,
        ResponseTemplate::new(200).set_body_json(json!([
            record(217.01, Some("Fora"), None),
            record(214.0, Some(""), Some("\r\n")),
        ])),
    )
    .await;
    assert!(client(&server).fetch(&query()).await.unwrap().is_none());
}

#[tokio::test]
async fn get_com_erro_http_ou_json_ainda_tenta_search() {
    for response in [
        ResponseTemplate::new(503),
        ResponseTemplate::new(200).set_body_string("not json"),
    ] {
        let server = MockServer::start().await;
        get(&server, response).await;
        search(
            &server,
            ResponseTemplate::new(200).set_body_json(json!([record(214.0, Some("Letra"), None)])),
        )
        .await;
        assert_eq!(
            client(&server)
                .fetch(&query())
                .await
                .unwrap()
                .unwrap()
                .plain
                .as_deref(),
            Some("Letra")
        );
    }
}

#[tokio::test]
async fn erros_do_provedor_sao_distintos_de_ausencia() {
    let server = MockServer::start().await;
    get(&server, ResponseTemplate::new(404)).await;
    search(&server, ResponseTemplate::new(429)).await;
    assert_eq!(
        client(&server).fetch(&query()).await.unwrap_err().kind(),
        "lyrics_http"
    );
}

#[tokio::test]
async fn consulta_sem_album_e_busca_404() {
    let server = MockServer::start().await;
    Mock::given(path("/api/get"))
        .and(query_param("album_name", ""))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;
    search(&server, ResponseTemplate::new(404)).await;
    let mut query = query();
    query.album = None;
    assert!(client(&server).fetch(&query).await.unwrap().is_none());
}

#[tokio::test]
async fn consulta_incompleta_nao_faz_rede() {
    let server = MockServer::start().await;
    let client = client(&server);
    for duration in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut q = query();
        q.duration_s = duration;
        assert!(client.fetch(&q).await.unwrap().is_none());
    }
    let mut q = query();
    q.artist = "  ".into();
    assert!(client.fetch(&q).await.unwrap().is_none());
    q = query();
    q.title.clear();
    assert!(client.fetch(&q).await.unwrap().is_none());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test(start_paused = true)]
async fn limita_lrclib_a_5_requisicoes_por_segundo() {
    let limiter = LyricsClient::limiter();
    limiter.acquire().await;
    let start = tokio::time::Instant::now();
    limiter.acquire().await;
    assert_eq!(start.elapsed(), std::time::Duration::from_millis(200));
}

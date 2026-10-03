use super::*;
use serde_json::json;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};
fn fixture(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/http/deezer")
                .join(format!("{name}.json")),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn urls_classify_all_supported_forms_and_reject_lookalikes() {
    for u in [
        "https://deezer.com/playlist/123",
        "https://www.deezer.com/br/album/123",
        "https://deezer.com/en/track/123?x=1",
        "https://deezer.com/artist/6160",
        "https://link.deezer.com/s/hello",
        "https://open.spotify.com/playlist/ABC123?si=x",
        "https://open.spotify.com/intl-pt/album/ABC123",
        "https://open.spotify.com/track/ABC123",
        "http://deezer.com/album/123",
    ] {
        assert!(classify(u).is_some(), "{u}");
    }
    for u in [
        "https://deezer.com.evil.example/album/123",
        "https://open.spotify.com.evil/track/ABC",
        "ftp://deezer.com/track/1",
        "https://user:pass@deezer.com/track/1",
        "https://deezer.com/album/not-a-number",
        "https://open.spotify.com/track/a/b",
        "https://deezer.com",
        "not a URL",
    ] {
        assert!(classify(u).is_none(), "{u}");
    }
}
#[tokio::test]
async fn deezer_album_and_playlist_paginate_and_map_source_metadata() {
    let server = MockServer::start().await;
    Mock::given(path("/album/301775"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("album")))
        .mount(&server)
        .await;
    Mock::given(path("/album/301775/tracks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("album-tracks")))
        .mount(&server)
        .await;
    let mut page = fixture("playlist-tracks");
    let values = page["data"].as_array().unwrap().clone();
    assert!(values.iter().all(|v| v["isrc"].is_string()));
    page["data"] = json!(&values[..2]);
    page["next"] = json!(format!("{}/page-two", server.uri()));
    Mock::given(path("/playlist/5207214368"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("playlist")))
        .mount(&server)
        .await;
    Mock::given(path("/playlist/5207214368/tracks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page))
        .mount(&server)
        .await;
    Mock::given(path("/page-two"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"data":&values[2..],"total":values.len()})),
        )
        .mount(&server)
        .await;
    let api = DeezerSource::new(&server.uri());
    let album = api.fetch("https://deezer.com/album/301775").await.unwrap();
    assert_eq!(album.tracks.len(), 16);
    assert_eq!(album.tracks[0].fields.track_no, Some(1));
    assert_eq!(album.tracks[0].fields.disc_no, Some(1));
    assert_eq!(album.tracks[0].fields.album.as_deref(), Some("Homework"));
    assert!(album.tracks[0].isrc.is_some());
    let playlist = api
        .fetch("https://deezer.com/playlist/5207214368")
        .await
        .unwrap();
    assert_eq!(playlist.tracks.len(), values.len());
    assert!(playlist.checksum.is_some());
}
#[tokio::test]
async fn deezer_error_800_and_pagination_origin_are_rejected() {
    let s = MockServer::start().await;
    Mock::given(path("/playlist/0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("error")))
        .mount(&s)
        .await;
    let api = DeezerSource::new(&s.uri());
    assert_eq!(
        api.fetch("https://deezer.com/playlist/0")
            .await
            .unwrap_err()
            .kind(),
        "deezer_not_found"
    );
    assert_eq!(
        api.get("https://example.com/tracks")
            .await
            .unwrap_err()
            .kind(),
        "import_response"
    );
}
#[tokio::test]
async fn short_link_resolves_to_validated_collection() {
    let s = MockServer::start().await;
    Mock::given(path("/short"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("Location", "https://www.deezer.com/album/301775"),
        )
        .mount(&s)
        .await;
    let api = DeezerSource::new(&s.uri());
    let k = api
        .resolve_short(&format!("{}/short", s.uri()))
        .await
        .unwrap();
    assert_eq!(k.id, "301775");
    assert_eq!(k.kind, "album");
}
async fn spotify() -> (MockServer, SpotifySource) {
    let s = MockServer::start().await;
    let settings = Arc::new(
        SettingsService::new(
            crate::Db::open_in_memory().unwrap(),
            Arc::new(crate::MemorySink::new()),
        )
        .await
        .unwrap(),
    );
    settings
        .update(
            serde_json::from_value(
                json!({"spotifyClientId":"fixture-id","spotifyClientSecret":"fixture-secret"}),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let api = SpotifySource::with_endpoints(settings, &s.uri(), &s.uri());
    (s, api)
}
async fn token_mock(s: &MockServer, ttl: u32) {
    let mut token: Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/http/spotify/token.json"
    ))
    .unwrap();
    token["expires_in"] = json!(ttl);
    Mock::given(method("POST"))
        .and(path("/api/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(token))
        .mount(s)
        .await;
}
#[tokio::test]
async fn spotify_token_refresh_pagination_and_track_fields() {
    let (s, api) = spotify().await;
    token_mock(&s, 3600).await;
    let track = json!({"id":"TRACK1","name":"Song","artists":[{"name":"Artist"}],"album":{"name":"Album","release_date":"1987-01-01","images":[{"url":"https://image.example/cover"}]},"duration_ms":214000,"external_ids":{"isrc":"GBARL9300135"},"track_number":2,"disc_number":1});
    Mock::given(path("/v1/playlists/PLAYLIST1"))
        .and(header("Authorization", "Bearer fixture-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"name":"Playlist"})))
        .expect(3)
        .mount(&s)
        .await;
    Mock::given(path("/v1/playlists/PLAYLIST1/items"))
        .and(query_param("limit", "50"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                json!({"items":[{"track":track}],"next":format!("{}/next",s.uri())}),
            ),
        )
        .mount(&s)
        .await;
    Mock::given(path("/next"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"items":[],"next":null})))
        .mount(&s)
        .await;
    let c = api
        .fetch("https://open.spotify.com/playlist/PLAYLIST1")
        .await
        .unwrap();
    assert_eq!(c.tracks[0].isrc.as_deref(), Some("GBARL9300135"));
    assert_eq!(c.tracks[0].duration_s, Some(214.));
    assert_eq!(c.tracks[0].fields.track_no, Some(2));
    api.fetch("https://open.spotify.com/playlist/PLAYLIST1")
        .await
        .unwrap();
    assert_eq!(
        s.received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| r.url.path() == "/api/token")
            .count(),
        1
    );
    api.token.lock().await.as_mut().unwrap().expires = Instant::now() - Duration::from_secs(1);
    api.fetch("https://open.spotify.com/playlist/PLAYLIST1")
        .await
        .unwrap();
    assert_eq!(
        s.received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| r.url.path() == "/api/token")
            .count(),
        2
    );
}
#[tokio::test]
async fn spotify_restricted_and_missing_credentials() {
    let (s, api) = spotify().await;
    token_mock(&s, 3600).await;
    Mock::given(path("/v1/playlists/PRIVATE"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&s)
        .await;
    assert_eq!(
        api.fetch("https://open.spotify.com/playlist/PRIVATE")
            .await
            .unwrap_err()
            .kind(),
        "spotify_playlist_restricted"
    );
    api.settings
        .update(serde_json::from_value(json!({"spotifyClientSecret":""})).unwrap())
        .await
        .unwrap();
    assert_eq!(
        api.token().await.unwrap_err().kind(),
        "spotify_credentials_missing"
    );
}
#[tokio::test]
async fn spotify_rate_limit_waits_retry_after() {
    let (s, mut api) = spotify().await;
    let delays = Arc::new(std::sync::Mutex::new(Vec::new()));
    api.retry_delays = Some(delays.clone());
    token_mock(&s, 3600).await;
    Mock::given(path("/v1/tracks/ONE"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "1"))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&s)
        .await;
    Mock::given(path("/v1/tracks/ONE"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"ONE"})))
        .with_priority(2)
        .mount(&s)
        .await;
    api.get("/v1/tracks/ONE", false).await.unwrap();
    assert_eq!(*delays.lock().unwrap(), vec![Duration::from_secs(1)]);
}

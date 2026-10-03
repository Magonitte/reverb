use super::*;
use serde_json::json;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

const ARCHIVE: &str = include_str!("../../../../tests/fixtures/http/f14/archive-metadata.json");
const ALBUM: &str = include_str!("../../../../tests/fixtures/http/f14/bandcamp-album.html");
const DOWNLOAD: &str = include_str!("../../../../tests/fixtures/http/f14/bandcamp-download.html");
const TRACK: &str = include_str!("../../../../tests/fixtures/http/f14/bandcamp-track.html");
const TRACK_DOWNLOAD: &str =
    include_str!("../../../../tests/fixtures/http/f14/bandcamp-track-download.html");

#[test]
fn url_table() {
    for (url, provider, collection) in [
        ("https://soundcloud.com/artist/track", "soundcloud", false),
        (
            "https://soundcloud.com/artist/sets/album",
            "soundcloud",
            true,
        ),
        ("https://artist.bandcamp.com/album/free", "bandcamp", true),
        ("https://artist.bandcamp.com/track/free", "bandcamp", false),
        (
            "https://archive.org/details/OpenGoldbergVariations",
            "archive",
            true,
        ),
        (
            "https://archive.org/details/OpenGoldbergVariations?file=01.flac",
            "archive",
            false,
        ),
        ("https://www.jamendo.com/track/123/title", "jamendo", false),
    ] {
        let found = matches(url).unwrap();
        assert_eq!(provider_id(url), provider);
        assert_eq!(matches!(found, UrlKind::Collection { .. }), collection);
        assert_eq!(crate::urlkind::classify(url), found);
    }
    for url in [
        "ftp://archive.org/details/a",
        "https://archive.org.evil.com/details/a",
        "https://artist.bandcamp.com.evil.com/track/a",
        "https://soundcloud.com/",
        "https://jamendo.com/track/not-a-number",
        "https://user:pass@archive.org/details/a",
    ] {
        assert!(matches(url).is_none(), "{url}");
    }
}

#[test]
fn archive_parsers_choose_original_flac_and_keep_track_tags() {
    let tracks =
        parsers::archive(ARCHIVE, "OpenGoldbergVariations", "https://archive.org").unwrap();
    assert_eq!(tracks.len(), 31);
    assert_eq!(tracks[0].title, "Aria");
    assert_eq!(tracks[0].artist.as_deref(), Some("Kimiko Ishizaka"));
    assert_eq!(tracks[0].track_no, Some(1));
    assert_eq!(tracks[0].ext, "flac");
    assert!(tracks[0].download_url.contains("%20"));
    assert_eq!(
        parsers::archive_search(
            include_str!("../../../../tests/fixtures/http/f14/archive-search.json"),
            "https://archive.org"
        )
        .unwrap()[0]
            .id,
        "OpenGoldbergVariations"
    );
    let restricted = json!({"is_dark":true,"files":[]});
    assert!(parsers::archive(&restricted.to_string(), "x", "https://archive.org").is_err());
}

#[test]
fn archive_replacement_cc0_keeps_24bit_flac_and_collection_metadata() {
    let tracks = parsers::archive(
        include_str!("../../../../tests/fixtures/http/f14/archive-replacement-metadata.json"),
        "bach-well-tempered-clavier-book-1",
        "https://archive.org",
    )
    .unwrap();
    assert_eq!(tracks.len(), 48);
    assert_eq!(tracks[0].title, "Prelude No. 1 in C major, BWV 846");
    assert_eq!(tracks[0].artist.as_deref(), Some("Kimiko Ishizaka"));
    assert_eq!(tracks[0].year, Some(2015));
    assert_eq!(tracks[0].track_no, Some(1));
    assert_eq!(tracks[0].ext, "flac");
    assert_eq!(
        parsers::archive_search(
            include_str!("../../../../tests/fixtures/http/f14/archive-replacement-search.json"),
            "https://archive.org"
        )
        .unwrap()[0]
            .id,
        "bach-well-tempered-clavier-book-1"
    );
}

#[test]
fn jamendo_download_permission_is_required() {
    let input = json!({"headers":{"status":"success"},"results":[{"id":"1","name":"Allowed","artist_name":"Artist","album_name":"Album","audiodownload_allowed":true,"audiodownload":"https://storage.jamendo.com/a.flac"},{"id":"2","name":"Blocked","audiodownload_allowed":false,"audiodownload":"https://storage.jamendo.com/b.flac"},{"id":"3","name":"Missing permission","audiodownload":"https://storage.jamendo.com/c.flac"}]});
    let tracks = parsers::jamendo(&input.to_string()).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].id, "1");
    assert!(parsers::jamendo("{\"headers\":{\"status\":\"failed\"},\"results\":[]}").is_err());
}

#[test]
fn bandcamp_only_free_downloads_without_email() {
    assert_eq!(
        parsers::bandcamp_status("var _statDL_result = { result: 'ok'};").unwrap()["result"],
        "ok"
    );
    let album = parsers::attribute_json(ALBUM, "data-tralbum").unwrap();
    let blob = parsers::attribute_json(DOWNLOAD, "data-blob").unwrap();
    assert!(parsers::bandcamp_free_page(&album)
        .unwrap()
        .starts_with("https://bandcamp.com/download?"));
    let tracks = parsers::bandcamp(
        &album,
        &blob,
        "https://soundslikeanearful.bandcamp.com/album/creative-commons-vol-1",
    )
    .unwrap();
    assert_eq!(tracks.len(), 26);
    assert_eq!(tracks[0].title, "Mellow Harmonics");
    let track = parsers::attribute_json(TRACK, "data-tralbum").unwrap();
    let download = parsers::attribute_json(TRACK_DOWNLOAD, "data-blob").unwrap();
    let tracks = parsers::bandcamp(
        &track,
        &download,
        "https://soundslikeanearful.bandcamp.com/track/mellow-harmonics",
    )
    .unwrap();
    assert_eq!(tracks.len(), 1);
    assert!(tracks[0].download_url.contains("enc=flac"));
    for album in [
        json!({}),
        json!({"require_email":true,"freeDownloadPage":"https://bandcamp.com/download?id=1"}),
        json!({"freeDownloadPage":"https://evil.example/download"}),
    ] {
        assert!(parsers::bandcamp_free_page(&album).is_err());
    }
    assert!(parsers::attribute_json("<html>changed</html>", "data-tralbum").is_err());
    let paid = json!({"download_items":[{"payment_type":"paid","downloads":{"flac":{"url":"https://bandcamp.com/a"}}}]});
    assert!(parsers::bandcamp(&track, &paid, "https://a.bandcamp.com/track/x").is_err());
}

#[tokio::test]
async fn http_progress_cancel_and_cleanup() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/audio"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![3u8; 32000]))
        .mount(&server)
        .await;
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("audio.flac");
    let events = std::sync::Mutex::new(Vec::new());
    download_http(
        &reqwest::Client::new(),
        &format!("{}/audio", server.uri()),
        &file,
        &|e| events.lock().unwrap().push(e),
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(&file).unwrap().len(), 32000);
    assert!(events.lock().unwrap().last().unwrap().finished);
    assert!(!file.with_extension("part").exists());
    Mock::given(path("/slow"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(10))
                .set_body_bytes(vec![1; 64]),
        )
        .mount(&server)
        .await;
    let token = CancellationToken::new();
    let clone = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(40)).await;
        clone.cancel();
    });
    let cancelled = tmp.path().join("cancel.flac");
    let result = download_http(
        &reqwest::Client::new(),
        &format!("{}/slow", server.uri()),
        &cancelled,
        &|_| {},
        &token,
    )
    .await;
    assert_eq!(result.unwrap_err().kind, ErrorKind::Cancelled);
    assert!(!cancelled.exists());
    assert!(!cancelled.with_extension("part").exists());
}

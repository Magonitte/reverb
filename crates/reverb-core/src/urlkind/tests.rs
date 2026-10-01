use super::*;

fn vid(id: &str, host: &str, hint: bool) -> UrlKind {
    UrlKind::Video {
        source_id: id.into(),
        url: format!("https://{host}/watch?v={id}"),
        playlist_hint: hint,
    }
}

fn coll(url: &str) -> UrlKind {
    UrlKind::Collection { url: url.into() }
}

const ALBUM: &str = "OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE";

#[test]
fn videos_validos() {
    let www = "www.youtube.com";
    let casos = [
        (
            "https://youtu.be/dQw4w9WgXcQ",
            vid("dQw4w9WgXcQ", www, false),
        ),
        (
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42",
            vid("dQw4w9WgXcQ", www, false),
        ),
        (
            "https://www.youtube.com/watch?si=abc&v=dQw4w9WgXcQ",
            vid("dQw4w9WgXcQ", www, false),
        ),
        (
            "https://m.youtube.com/watch?v=dQw4w9WgXcQ",
            vid("dQw4w9WgXcQ", www, false),
        ),
        (
            "https://www.youtube.com/shorts/aqz-KE-bpKQ",
            vid("aqz-KE-bpKQ", www, false),
        ),
        (
            "https://www.youtube.com/live/jfKfPfyJRdk",
            vid("jfKfPfyJRdk", www, false),
        ),
        (
            "https://music.youtube.com/watch?v=lYBUbBu4W08",
            vid("lYBUbBu4W08", "music.youtube.com", false),
        ),
        (
            "HTTPS://WWW.YOUTUBE.COM/watch?v=dQw4w9WgXcQ",
            vid("dQw4w9WgXcQ", www, false),
        ),
        (
            "  https://youtu.be/dQw4w9WgXcQ  ",
            vid("dQw4w9WgXcQ", www, false),
        ),
        (
            "https://youtu.be/dQw4w9WgXcQ?si=xyz",
            vid("dQw4w9WgXcQ", www, false),
        ),
    ];
    for (input, expected) in casos {
        assert_eq!(classify(input), expected, "{input}");
    }
}

#[test]
fn video_com_dica_de_playlist() {
    let input = format!("https://www.youtube.com/watch?v=lYBUbBu4W08&list={ALBUM}");
    assert_eq!(
        classify(&input),
        vid("lYBUbBu4W08", "www.youtube.com", true)
    );
}

#[test]
fn colecoes() {
    let album = format!("https://www.youtube.com/playlist?list={ALBUM}");
    let casos = [
        (album.clone(), album.clone()),
        (
            format!("https://music.youtube.com/playlist?list={ALBUM}"),
            album.clone(),
        ),
        (
            "https://music.youtube.com/browse/MPREb_dcYZhAh5urI".into(),
            "https://music.youtube.com/browse/MPREb_dcYZhAh5urI".into(),
        ),
        (
            "https://www.youtube.com/@RickAstleyYT".into(),
            "https://www.youtube.com/@RickAstleyYT/videos".into(),
        ),
        (
            "https://www.youtube.com/@RickAstleyYT/videos".into(),
            "https://www.youtube.com/@RickAstleyYT/videos".into(),
        ),
        (
            "https://www.youtube.com/@RickAstleyYT/".into(),
            "https://www.youtube.com/@RickAstleyYT/videos".into(),
        ),
        (
            "https://www.youtube.com/channel/UCuAXFkgsw1L7xaCfnd5JJOw".into(),
            "https://www.youtube.com/channel/UCuAXFkgsw1L7xaCfnd5JJOw/videos".into(),
        ),
        (
            "https://www.youtube.com/c/RickAstley".into(),
            "https://www.youtube.com/c/RickAstley/videos".into(),
        ),
        (
            "https://www.youtube.com/user/RickAstley/shorts".into(),
            "https://www.youtube.com/user/RickAstley/shorts".into(),
        ),
        (format!("https://www.youtube.com/watch?list={ALBUM}"), album),
    ];
    for (input, expected) in casos {
        assert_eq!(classify(&input), coll(&expected), "{input}");
    }
}

#[test]
fn buscas() {
    for text in ["rick astley", "never gonna give you up", "  lo-fi: beats  "] {
        assert_eq!(
            classify(text),
            UrlKind::Search {
                query: text.trim().into()
            },
            "{text}"
        );
    }
}

#[test]
fn nao_suportadas() {
    for input in [
        "https://vimeo.com/123",
        "ftp://youtube.com/watch?v=x",
        "https://youtube.com.evil.com/watch?v=x",
        "https://www.youtube.com/",
        "javascript:alert(1)",
        "",
        "   ",
        "https://www.youtube.com/watch?v=curto",
        "https://www.youtube.com/watch",
        "https://www.youtube.com/playlist",
        "https://www.youtube.com/@",
        "https://notyoutube.com/watch?v=dQw4w9WgXcQ",
        "https://soundcloud.com/artista/faixa",
    ] {
        assert_eq!(classify(input), UrlKind::Unsupported, "{input:?}");
    }
}

#[test]
fn serializa_com_kind_e_camel_case() {
    let json = serde_json::to_value(vid("dQw4w9WgXcQ", "www.youtube.com", true)).unwrap();
    assert_eq!(json["kind"], "video");
    assert_eq!(json["sourceId"], "dQw4w9WgXcQ");
    assert_eq!(json["playlistHint"], true);
}

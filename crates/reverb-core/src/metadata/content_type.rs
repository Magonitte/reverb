//! Música ou outra coisa? (arquitetura §11, passo 1).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ytdlp::VideoInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ContentType {
    Music,
    Other,
}

impl ContentType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Music => "music",
            Self::Other => "other",
        }
    }
}

/// A URL é do `music.youtube.com`?
pub fn is_music_host(url: &str) -> bool {
    url::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_ascii_lowercase))
        .is_some_and(|host| host == "music.youtube.com")
}

/// `music` se: há `track`; as categorias contêm `Music`; o canal termina em " - Topic"; o
/// uploader contém `VEVO`; ou a URL de origem é do `music.youtube.com`.
pub fn content_type(info: &VideoInfo, source_url: Option<&str>) -> ContentType {
    let has_track = info.track.is_some();
    let in_category = info
        .categories
        .iter()
        .any(|c| c.trim().eq_ignore_ascii_case("music"));
    let topic_channel = info
        .channel
        .as_deref()
        .is_some_and(|c| c.trim().to_ascii_lowercase().ends_with(" - topic"));
    let vevo = info
        .uploader
        .as_deref()
        .is_some_and(|u| u.to_ascii_uppercase().contains("VEVO"));
    let music_url = [source_url, info.webpage_url.as_deref()]
        .into_iter()
        .flatten()
        .any(is_music_host);
    if has_track || in_category || topic_channel || vevo || music_url {
        ContentType::Music
    } else {
        ContentType::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> VideoInfo {
        let path = format!(
            "{}/../../tests/fixtures/ytdlp/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        VideoInfo::from_json(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn bare() -> VideoInfo {
        let mut info = fixture("fx1-video.json");
        info.categories.clear();
        info.channel = Some("canal".into());
        info.uploader = Some("canal".into());
        info.track = None;
        info.webpage_url = Some("https://www.youtube.com/watch?v=x".into());
        info
    }

    #[test]
    fn fx1_nao_e_musica() {
        assert_eq!(
            content_type(&fixture("fx1-video.json"), None),
            ContentType::Other
        );
    }

    #[test]
    fn fx2_e_musica_pela_faixa_oficial() {
        assert_eq!(
            content_type(&fixture("fx2-music.json"), None),
            ContentType::Music
        );
    }

    #[test]
    fn fx3_e_musica_pela_categoria() {
        let info = fixture("fx3-clip.json");
        assert!(info.track.is_none());
        assert_eq!(content_type(&info, None), ContentType::Music);
    }

    #[test]
    fn canal_topic_e_musica() {
        let mut info = bare();
        info.channel = Some("Rick Astley - Topic".into());
        assert_eq!(content_type(&info, None), ContentType::Music);
    }

    #[test]
    fn vevo_e_musica() {
        let mut info = bare();
        info.uploader = Some("RickAstleyVEVO".into());
        assert_eq!(content_type(&info, None), ContentType::Music);
    }

    #[test]
    fn url_do_youtube_music_e_musica() {
        let info = bare();
        assert_eq!(content_type(&info, None), ContentType::Other);
        assert_eq!(
            content_type(&info, Some("https://music.youtube.com/watch?v=x")),
            ContentType::Music
        );
        assert_eq!(
            content_type(&info, Some("https://www.youtube.com/watch?v=x")),
            ContentType::Other
        );
    }

    #[test]
    fn so_a_categoria_music_conta() {
        let mut info = bare();
        info.categories = vec!["Entertainment".into(), "Gaming".into()];
        assert_eq!(content_type(&info, None), ContentType::Other);
        info.categories.push("music".into());
        assert_eq!(content_type(&info, None), ContentType::Music);
    }
}

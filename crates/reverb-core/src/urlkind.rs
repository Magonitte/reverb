//! Classificação de URLs e textos digitados (arquitetura §14). Função pura, sem rede.

use serde::Serialize;
use ts_rs::TS;
use url::Url;

/// Resultado de `classify`. `url` é sempre a forma normalizada que o yt-dlp deve receber.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum UrlKind {
    /// Um vídeo/faixa. `playlist_hint` = a URL também trazia `list=`.
    #[serde(rename_all = "camelCase")]
    Video {
        source_id: String,
        url: String,
        playlist_hint: bool,
    },
    /// Playlist, álbum ou canal.
    Collection {
        url: String,
    },
    /// Texto sem esquema: vira busca.
    Search {
        query: String,
    },
    Unsupported,
}

const CHANNEL_TABS: &[&str] = &[
    "videos",
    "shorts",
    "streams",
    "playlists",
    "releases",
    "podcasts",
    "featured",
    "community",
    "search",
    "about",
];

fn is_video_id(text: &str) -> bool {
    text.len() == 11
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// `esquema:` seguido de texto sem espaços (`javascript:alert(1)`, `ftp://…`).
fn looks_like_url(text: &str) -> bool {
    if text.chars().any(char::is_whitespace) {
        return false;
    }
    let Some((scheme, _)) = text.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
}

fn query_param(url: &Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(key, value)| key == name && !value.is_empty())
        .map(|(_, value)| value.into_owned())
}

fn video(host: &str, id: &str, playlist_hint: bool) -> UrlKind {
    if !is_video_id(id) {
        return UrlKind::Unsupported;
    }
    let site = if host == "music.youtube.com" {
        "music.youtube.com"
    } else {
        "www.youtube.com"
    };
    UrlKind::Video {
        source_id: id.to_string(),
        url: format!("https://{site}/watch?v={id}"),
        playlist_hint,
    }
}

fn playlist(list: &str) -> UrlKind {
    UrlKind::Collection {
        url: format!("https://www.youtube.com/playlist?list={list}"),
    }
}

fn classify_youtube(host: &str, url: &Url) -> UrlKind {
    let segments: Vec<&str> = url
        .path_segments()
        .map(|s| s.filter(|part| !part.is_empty()).collect())
        .unwrap_or_default();
    let list = query_param(url, "list");

    match segments.as_slice() {
        ["watch"] => match (query_param(url, "v"), list) {
            (Some(id), list) => video(host, &id, list.is_some()),
            (None, Some(list)) => playlist(&list),
            (None, None) => UrlKind::Unsupported,
        },
        ["shorts" | "live" | "embed", id] => video(host, id, false),
        ["playlist"] => list.map_or(UrlKind::Unsupported, |l| playlist(&l)),
        ["browse", id] if id.starts_with("MPREb_") => UrlKind::Collection {
            url: format!("https://{host}/browse/{id}"),
        },
        [first, rest @ ..] if is_channel_root(first, rest) => {
            let (base, tab) = channel_base(&segments);
            let tab = tab.unwrap_or("videos");
            UrlKind::Collection {
                url: format!("https://{host}/{base}/{tab}"),
            }
        }
        _ => UrlKind::Unsupported,
    }
}

fn is_channel_root(first: &str, rest: &[&str]) -> bool {
    if first.starts_with('@') {
        return first.len() > 1;
    }
    matches!(first, "channel" | "c" | "user") && !rest.is_empty()
}

/// Separa `@handle` ou `channel/UC…` da aba (`videos`, …) quando existir.
fn channel_base<'a>(segments: &[&'a str]) -> (String, Option<&'a str>) {
    let base_len = if segments[0].starts_with('@') { 1 } else { 2 };
    let base = segments[..base_len].join("/");
    let tab = segments
        .get(base_len)
        .copied()
        .filter(|tab| CHANNEL_TABS.contains(tab));
    (base, tab)
}

/// Classifica o que o usuário colou ou digitou.
pub fn classify(input: &str) -> UrlKind {
    crate::sources::matches(input.trim()).unwrap_or_else(|| classify_youtube_input(input))
}

pub(crate) fn classify_youtube_input(input: &str) -> UrlKind {
    let text = input.trim();
    if text.is_empty() {
        return UrlKind::Unsupported;
    }
    if !looks_like_url(text) {
        return UrlKind::Search {
            query: text.to_string(),
        };
    }
    let Ok(url) = Url::parse(text) else {
        return UrlKind::Unsupported;
    };
    if !matches!(url.scheme(), "http" | "https") {
        return UrlKind::Unsupported;
    }
    let Some(host) = url.host_str().map(str::to_ascii_lowercase) else {
        return UrlKind::Unsupported;
    };
    match host.as_str() {
        "youtu.be" => match url.path_segments().and_then(|mut s| s.next()) {
            Some(id) => video("www.youtube.com", id, query_param(&url, "list").is_some()),
            None => UrlKind::Unsupported,
        },
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com" => {
            classify_youtube(&host, &url)
        }
        _ => UrlKind::Unsupported,
    }
}

#[cfg(test)]
mod tests;

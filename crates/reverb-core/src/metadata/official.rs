//! Versão oficial de uma faixa: ISRC primeiro, depois texto com os filtros eliminatórios, a
//! penalidade cumulativa de palavras de versão, artistas múltiplos e desempate (estudo E1,
//! arquitetura §11 passo 3).

use std::collections::BTreeSet;
use std::sync::LazyLock;

use futures_util::future::join_all;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use ts_rs::TS;

use super::normalize::{norm, norm_plain};
use super::parse_title::{clean_channel, parse_title, split_artists};
use super::score::{combine, duration_part, sim, version_mismatch, DurationTolerance, Subject};
use crate::backend::DownloadBackend;
use crate::ytdlp::{Analysis, SearchResult, SearchSource, VideoInfo};

/// Corte para aceitar uma versão oficial.
pub const MIN_SCORE: f64 = 0.80;
/// Diferença de pontuação dentro da qual a faixa oficial vence no desempate.
pub const TIE_WINDOW: f64 = 0.08;
/// Penalidade no `title_sim` por palavra de versão que só o resultado tem.
pub const VERSION_WORD_PENALTY: f64 = 0.15;
/// Quantos resultados da busca são analisados em paralelo.
const TOP: u32 = 3;
/// Limites eliminatórios de duração (segundos): faixa × clipe.
const TRACK_MAX_DELTA: f64 = 15.0;
const CLIP_MAX_DELTA: f64 = 45.0;
const MIN_TITLE_SIM: f64 = 0.60;
const MIN_ARTIST_SIM: f64 = 0.70;
/// `sim` mínimo para considerar que um artista da busca foi encontrado.
const ARTIST_FOUND: f64 = 0.85;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum OfficialVia {
    Isrc,
    Text,
}

/// A faixa oficial encontrada para um vídeo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OfficialMatch {
    pub video_id: String,
    pub url: String,
    pub score: f64,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub isrc: Option<String>,
    pub via: OfficialVia,
}

/// O achado junto com a análise do vídeo oficial (já feita para decidir).
#[derive(Debug, Clone)]
pub struct OfficialFound {
    pub matched: OfficialMatch,
    pub info: VideoInfo,
}

// ---------------------------------------------------------------------------------------------
// Funções puras
// ---------------------------------------------------------------------------------------------

static ISRC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Z]{2}[A-Z0-9]{3}\d{7}$").expect("regex de ISRC"));

/// `^[A-Z]{2}[A-Z0-9]{3}\d{7}$` (12 caracteres, exatamente assim).
pub fn is_valid_isrc(text: &str) -> bool {
    ISRC.is_match(text)
}

/// Aceita `GB-ARL-93-00135` e minúsculas; devolve o ISRC canônico ou `None` se inválido.
pub fn normalize_isrc(text: &str) -> Option<String> {
    let cleaned: String = text
        .chars()
        .filter(|c| !matches!(c, '-' | ' '))
        .collect::<String>()
        .to_ascii_uppercase();
    is_valid_isrc(&cleaned).then_some(cleaned)
}

static VERSION_WORDS: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    [
        ("live", r"\b(?:live|ao vivo)\b"),
        ("remix", r"\bremix(?:es)?\b"),
        ("remaster", r"\bremaster(?:ed)?\b"),
        ("acoustic", r"\b(?:acoustic|acustico)\b"),
        ("instrumental", r"\binstrumental\b"),
        ("cover", r"\bcover\b"),
        ("slowed", r"\bslowed\b"),
        ("sped up", r"\bsped up\b"),
        ("reverb", r"\breverb\b"),
        ("bass boost", r"\bbass boost(?:ed)?\b"),
        ("8d", r"\b8d\b"),
        ("acapella", r"\b(?:acapella|a cappella)\b"),
        ("concert", r"\b(?:concert|show)\b"),
        ("karaoke", r"\bkaraoke\b"),
    ]
    .into_iter()
    .map(|(name, pattern)| {
        (
            name,
            Regex::new(pattern).expect("regex de palavra de versão"),
        )
    })
    .collect()
});

/// Palavras de versão presentes no texto (procura também dentro de parênteses).
pub fn version_words(text: &str) -> BTreeSet<&'static str> {
    let plain = norm_plain(text);
    VERSION_WORDS
        .iter()
        .filter(|(_, regex)| regex.is_match(&plain))
        .map(|(name, _)| *name)
        .collect()
}

/// 0,15 por palavra de versão que o resultado tem e a busca não (cumulativo).
pub fn version_word_penalty(query_title: &str, result_title: &str) -> f64 {
    let wanted = version_words(query_title);
    let extra = version_words(result_title).difference(&wanted).count();
    VERSION_WORD_PENALTY * extra as f64
}

fn contains_words(haystack: &str, needle: &str) -> bool {
    !needle.is_empty() && format!(" {haystack} ").contains(&format!(" {needle} "))
}

/// Fração dos artistas da busca que o resultado cobre. Um artista conta se algum artista do
/// resultado se parece com ele (`sim ≥ 0,85`), se aparece como palavras inteiras no **título** do
/// resultado (ex.: "feat. B") ou dentro de um nome único composto (ex.: "A, B & C"). Sem nenhum
/// encontrado, devolve a maior similaridade de par (limitada a 0,84).
pub fn multi_artist_sim(
    query_artists: &[String],
    result_artists: &[String],
    result_title: &str,
) -> f64 {
    if query_artists.is_empty() || result_artists.is_empty() {
        return 0.0;
    }
    let title = norm_plain(result_title);
    let names: Vec<(String, String)> = result_artists
        .iter()
        .map(|a| (norm(a), norm_plain(a)))
        .collect();
    let mut found = 0usize;
    let mut best_pair: f64 = 0.0;
    for artist in query_artists {
        let wanted = norm(artist);
        let wanted_plain = norm_plain(artist);
        let pair = names
            .iter()
            .map(|(candidate, _)| {
                if wanted.is_empty() || candidate.is_empty() {
                    0.0
                } else {
                    strsim::jaro_winkler(&wanted, candidate)
                }
            })
            .fold(0.0, f64::max);
        best_pair = best_pair.max(pair);
        let in_names = names
            .iter()
            .any(|(_, plain)| contains_words(plain, &wanted_plain));
        if pair >= ARTIST_FOUND || contains_words(&title, &wanted_plain) || in_names {
            found += 1;
        }
    }
    if found == 0 {
        best_pair.min(0.84)
    } else {
        found as f64 / query_artists.len() as f64
    }
}

/// Por que um resultado foi descartado antes da média.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elimination {
    NoCommonWord,
    TitleSimilarity,
    ArtistSimilarity,
    Duration,
}

/// `title_sim` com a penalidade cumulativa de palavras de versão e `artist_sim` de artistas
/// múltiplos, as duas peças comparadas pelos filtros e pela média.
pub fn match_parts(query: &Subject, result: &Subject) -> (f64, f64) {
    let title_sim = (sim(&query.title, &result.title)
        - version_word_penalty(&query.title, &result.title))
    .max(0.0);
    let artist_sim = multi_artist_sim(&query.artists, &result.artists, &result.title);
    (title_sim, artist_sim)
}

/// Filtros eliminatórios (antes da média). `via_isrc` dispensa os de similaridade.
pub fn eliminatory_filters(
    query: &Subject,
    result: &Subject,
    tolerance: DurationTolerance,
    via_isrc: bool,
) -> Option<Elimination> {
    let wanted = norm(&query.title);
    let got = norm(&result.title);
    let wanted_words: BTreeSet<&str> = wanted.split_whitespace().collect();
    if !got.split_whitespace().any(|w| wanted_words.contains(w)) {
        return Some(Elimination::NoCommonWord);
    }
    if let (Some(a), Some(b)) = (query.duration_s, result.duration_s) {
        let limit = match tolerance {
            DurationTolerance::Normal => TRACK_MAX_DELTA,
            DurationTolerance::Clip => CLIP_MAX_DELTA,
        };
        if (a - b).abs() > limit {
            return Some(Elimination::Duration);
        }
    }
    if !via_isrc {
        let (title_sim, artist_sim) = match_parts(query, result);
        if title_sim < MIN_TITLE_SIM {
            return Some(Elimination::TitleSimilarity);
        }
        if artist_sim < MIN_ARTIST_SIM {
            return Some(Elimination::ArtistSimilarity);
        }
    }
    None
}

/// Pontuação de um resultado já aprovado nos filtros (§11.2 com as peças do E1).
pub fn match_score(query: &Subject, result: &Subject, tolerance: DurationTolerance) -> f64 {
    let (title_sim, artist_sim) = match_parts(query, result);
    combine(
        title_sim,
        artist_sim,
        duration_part(query.duration_s, result.duration_s, tolerance),
        version_mismatch(&query.title, &result.title),
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scored {
    pub score: f64,
    /// Faixa oficial ("verificada") do catálogo.
    pub official: bool,
}

/// Desempate: entre os que ficam a até 0,08 do melhor, vence a faixa oficial; dentro do mesmo
/// grupo, a maior pontuação. Devolve o índice do escolhido.
pub fn tie_break(items: &[Scored]) -> Option<usize> {
    let top = items
        .iter()
        .map(|i| i.score)
        .fold(f64::NEG_INFINITY, f64::max);
    if !top.is_finite() {
        return None;
    }
    let window: Vec<usize> = (0..items.len())
        .filter(|&i| top - items[i].score <= TIE_WINDOW + 1e-12)
        .collect();
    let pool = if window.iter().any(|&i| items[i].official) {
        window
            .into_iter()
            .filter(|&i| items[i].official)
            .collect::<Vec<_>>()
    } else {
        window
    };
    pool.into_iter()
        .max_by(|&a, &b| items[a].score.total_cmp(&items[b].score))
}

// ---------------------------------------------------------------------------------------------
// Busca
// ---------------------------------------------------------------------------------------------

fn music_url(id: &str) -> String {
    format!("https://music.youtube.com/watch?v={id}")
}

/// Título, artistas e duração de um vídeo do YouTube (faixa oficial ou parse do título).
pub fn subject_of(info: &VideoInfo) -> Subject {
    if info.is_official_track {
        let artists = if info.artists.is_empty() {
            info.artist
                .as_deref()
                .map(split_artists)
                .unwrap_or_default()
        } else {
            info.artists.clone()
        };
        return Subject {
            title: info.track.clone().unwrap_or_else(|| info.title.clone()),
            artists,
            duration_s: info.duration,
        };
    }
    let parsed = parse_title(&info.title, info.channel.as_deref());
    Subject {
        title: parsed.title,
        artists: parsed.artists,
        duration_s: info.duration,
    }
}

/// Artistas do vídeo oficial já analisado, na forma de texto.
fn artist_text(info: &VideoInfo) -> String {
    if !info.artists.is_empty() {
        return info.artists.join(", ");
    }
    info.artist
        .clone()
        .or_else(|| info.channel.as_deref().and_then(clean_channel))
        .unwrap_or_default()
}

async fn analyze_top(
    backend: &dyn DownloadBackend,
    results: &[SearchResult],
    skip_id: &str,
    cancel: &CancellationToken,
) -> Vec<VideoInfo> {
    let tasks = results
        .iter()
        .filter(|r| r.id != skip_id)
        .take(TOP as usize)
        .map(|r| async move { backend.analyze(&music_url(&r.id), cancel).await.ok() });
    join_all(tasks)
        .await
        .into_iter()
        .flatten()
        .filter_map(|analysis| match analysis {
            Analysis::Video { info } => Some(*info),
            Analysis::Collection { .. } => None,
        })
        .collect()
}

fn found(info: VideoInfo, score: f64, isrc: Option<&str>, via: OfficialVia) -> OfficialFound {
    OfficialFound {
        matched: OfficialMatch {
            url: music_url(&info.id),
            video_id: info.id.clone(),
            score,
            title: info.track.clone().unwrap_or_else(|| info.title.clone()),
            artist: artist_text(&info),
            album: info.album.clone(),
            isrc: isrc.map(str::to_string),
            via,
        },
        info,
    }
}

/// Escolhe o melhor entre os analisados: filtros ⇒ pontuação ⇒ corte ⇒ desempate.
fn choose(
    query: &Subject,
    candidates: Vec<VideoInfo>,
    tolerance: DurationTolerance,
    via_isrc: bool,
    only_official: bool,
) -> Option<(VideoInfo, f64)> {
    let mut ranked: Vec<(VideoInfo, Scored)> = candidates
        .into_iter()
        .filter(|info| !only_official || info.is_official_track)
        .filter_map(|info| {
            let subject = subject_of(&info);
            if eliminatory_filters(query, &subject, tolerance, via_isrc).is_some() {
                return None;
            }
            let score = match_score(query, &subject, tolerance);
            (score >= MIN_SCORE).then(|| {
                let official = info.is_official_track;
                (info, Scored { score, official })
            })
        })
        .collect();
    let scored: Vec<Scored> = ranked.iter().map(|(_, s)| *s).collect();
    let index = tie_break(&scored)?;
    let (info, scored) = ranked.swap_remove(index);
    Some((info, scored.score))
}

/// `find_official_version`: ISRC primeiro (busca `<ISRC>` no YouTube Music), depois texto
/// (`"<artista> <título>"`, seção de músicas e, se nada servir, a de vídeos).
pub async fn find_official(
    backend: &dyn DownloadBackend,
    video: &VideoInfo,
    isrc: Option<&str>,
    cancel: &CancellationToken,
) -> Option<OfficialFound> {
    let query = subject_of(video);
    // A fonte já é a faixa oficial: nada a trocar. Clipe ⇒ tolerância de duração de clipe.
    if video.is_official_track {
        return None;
    }
    let tolerance = DurationTolerance::Clip;
    let isrc = isrc.and_then(normalize_isrc);

    if let Some(isrc) = isrc.as_deref() {
        let results = backend
            .search(SearchSource::YtMusic, isrc, TOP, cancel)
            .await
            .ok()
            .unwrap_or_default();
        let infos = analyze_top(backend, &results, &video.id, cancel).await;
        let officials: Vec<VideoInfo> = infos
            .iter()
            .filter(|info| info.is_official_track)
            .cloned()
            .collect();
        if officials.len() == 1 {
            let info = officials.into_iter().next()?;
            return Some(found(info, 1.0, Some(isrc), OfficialVia::Isrc));
        }
        if officials.len() > 1 {
            if let Some((info, score)) = choose(&query, officials, tolerance, true, true) {
                return Some(found(info, score, Some(isrc), OfficialVia::Isrc));
            }
        }
    }

    let text = match query.artists.first() {
        Some(artist) => format!("{artist} {}", query.title),
        None => query.title.clone(),
    };
    for (source, only_official) in [
        (SearchSource::YtMusic, false),
        (SearchSource::Youtube, true),
    ] {
        let results = backend
            .search(source, &text, TOP, cancel)
            .await
            .ok()
            .unwrap_or_default();
        let infos = analyze_top(backend, &results, &video.id, cancel).await;
        if let Some((info, score)) = choose(&query, infos, tolerance, false, only_official) {
            // O ISRC conhecido é da gravação de origem; esta faixa pode ser de outra versão.
            return Some(found(info, score, None, OfficialVia::Text));
        }
    }
    None
}

#[cfg(test)]
mod tests;

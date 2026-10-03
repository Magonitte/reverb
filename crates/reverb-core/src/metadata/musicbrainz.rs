//! Provedor MusicBrainz (sem chave, **1 requisição por segundo**, `User-Agent` identificável) e
//! capa pelo Cover Art Archive (apenas a URL candidata).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;

use super::provider::{get_json, year_of, Candidate, MetadataProvider, Query, RateLimiter};

pub const ID: &str = "musicbrainz";

/// Intervalo mínimo exigido pelo serviço.
pub const MIN_INTERVAL: Duration = Duration::from_secs(1);

pub struct MusicBrainz {
    client: reqwest::Client,
    base: String,
    cover_art: String,
    limiter: Arc<RateLimiter>,
}

impl MusicBrainz {
    pub fn limiter() -> Arc<RateLimiter> {
        RateLimiter::new(MIN_INTERVAL)
    }

    pub fn new(
        client: reqwest::Client,
        base: &str,
        cover_art: &str,
        limiter: Arc<RateLimiter>,
    ) -> Self {
        Self {
            client,
            base: base.trim_end_matches('/').to_string(),
            cover_art: cover_art.trim_end_matches('/').to_string(),
            limiter,
        }
    }
}

/// Aspas e barras têm sentido na sintaxe Lucene: viram espaço dentro das frases.
fn lucene(text: &str) -> String {
    text.chars()
        .map(|c| if matches!(c, '"' | '\\') { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn query_string(query: &Query) -> String {
    let recording = format!("recording:\"{}\"", lucene(&query.title));
    match query.artist() {
        Some(artist) => format!("{recording} AND artist:\"{}\"", lucene(artist)),
        None => recording,
    }
}

fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}

/// Escolhe o lançamento mais antigo (de preferência oficial) entre os da gravação.
fn pick_release(releases: &[Value]) -> Option<&Value> {
    let date_key = |r: &Value| text(r, "date").unwrap_or_else(|| "9999".to_string());
    let official = |r: &Value| r.get("status").and_then(Value::as_str) == Some("Official");
    releases
        .iter()
        .filter(|r| official(r))
        .min_by_key(|r| date_key(r))
        .or_else(|| releases.iter().min_by_key(|r| date_key(r)))
}

fn parse_recording(item: &Value, cover_art: &str) -> Option<Candidate> {
    let id = text(item, "id")?;
    let mut candidate = Candidate::new(ID, id.clone(), text(item, "title")?);
    candidate.mb_recording_id = Some(id);
    candidate.artists = item
        .get("artist-credit")
        .and_then(Value::as_array)
        .map(|credits| {
            credits
                .iter()
                .filter_map(|credit| {
                    credit
                        .get("artist")
                        .and_then(|a| text(a, "name"))
                        .or_else(|| text(credit, "name"))
                })
                .collect()
        })
        .unwrap_or_default();
    candidate.duration_s = item
        .get("length")
        .and_then(Value::as_f64)
        .filter(|ms| *ms > 0.0)
        .map(|ms| ms / 1000.0);
    candidate.year = text(item, "first-release-date")
        .as_deref()
        .and_then(year_of);
    let releases = item
        .get("releases")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    if let Some(release) = pick_release(releases) {
        candidate.album = text(release, "title");
        candidate.year = text(release, "date")
            .as_deref()
            .and_then(year_of)
            .or(candidate.year);
        if let Some(release_id) = text(release, "id") {
            candidate.cover_url = Some(format!("{cover_art}/release/{release_id}/front-1200"));
        }
        if let Some(medium) = release
            .get("media")
            .and_then(Value::as_array)
            .and_then(|media| media.first())
        {
            candidate.disc_no = medium
                .get("position")
                .and_then(Value::as_u64)
                .and_then(|n| u32::try_from(n).ok());
            candidate.track_total = medium
                .get("track-count")
                .and_then(Value::as_u64)
                .and_then(|n| u32::try_from(n).ok());
            candidate.track_no = medium
                .get("track")
                .and_then(Value::as_array)
                .and_then(|tracks| tracks.first())
                .and_then(|t| text(t, "number"))
                .and_then(|n| n.parse().ok());
        }
    }
    Some(candidate)
}

#[async_trait]
impl MetadataProvider for MusicBrainz {
    fn id(&self) -> &'static str {
        ID
    }

    async fn search(&self, query: &Query) -> Vec<Candidate> {
        let q = query_string(query);
        let url = format!("{}/ws/2/recording", self.base);
        let Some(value) = get_json(
            &self.client,
            &self.limiter,
            ID,
            &url,
            &[("query", q.as_str()), ("fmt", "json"), ("limit", "5")],
        )
        .await
        else {
            return Vec::new();
        };
        value
            .get("recordings")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| parse_recording(item, &self.cover_art))
                    .take(5)
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consulta_lucene_sem_aspas_internas() {
        let query = Query::new("Say \"Hi\"", vec!["A\\B".into()], None);
        assert_eq!(
            query_string(&query),
            "recording:\"Say Hi\" AND artist:\"A B\""
        );
        let sem_artista = Query::new("Título", Vec::new(), None);
        assert_eq!(query_string(&sem_artista), "recording:\"Título\"");
    }
}

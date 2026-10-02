//! Provedor iTunes Search (sem chave).

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use super::provider::{get_json, year_of, Candidate, MetadataProvider, Query, RateLimiter};

pub const ID: &str = "itunes";

pub struct Itunes {
    client: reqwest::Client,
    base: String,
    limiter: Arc<RateLimiter>,
}

impl Itunes {
    /// 3 requisições por segundo.
    pub fn limiter() -> Arc<RateLimiter> {
        RateLimiter::per_second(3)
    }

    pub fn new(client: reqwest::Client, base: &str, limiter: Arc<RateLimiter>) -> Self {
        Self {
            client,
            base: base.trim_end_matches('/').to_string(),
            limiter,
        }
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

fn number(value: &Value, key: &str) -> Option<u32> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .filter(|n| *n > 0)
}

/// `artworkUrl100` ⇒ 1200 px (troca o sufixo `100x100bb`).
pub fn large_artwork(url: &str) -> String {
    url.replace("100x100bb", "1200x1200bb")
}

pub(crate) fn parse_item(item: &Value) -> Option<Candidate> {
    if item.get("wrapperType").and_then(Value::as_str) == Some("collection") {
        return None;
    }
    if let Some(kind) = item.get("kind").and_then(Value::as_str) {
        if kind != "song" {
            return None;
        }
    }
    let id = item
        .get("trackId")
        .and_then(Value::as_u64)
        .map(|n| n.to_string())?;
    let mut candidate = Candidate::new(ID, id, text(item, "trackName")?);
    if let Some(artist) = text(item, "artistName") {
        candidate.artists.push(artist);
    }
    candidate.album = text(item, "collectionName");
    candidate.album_artist = text(item, "collectionArtistName");
    candidate.year = text(item, "releaseDate").as_deref().and_then(year_of);
    candidate.genre = text(item, "primaryGenreName");
    candidate.track_no = number(item, "trackNumber");
    candidate.track_total = number(item, "trackCount");
    candidate.disc_no = number(item, "discNumber");
    candidate.duration_s = item
        .get("trackTimeMillis")
        .and_then(Value::as_f64)
        .filter(|ms| *ms > 0.0)
        .map(|ms| ms / 1000.0);
    candidate.cover_url = text(item, "artworkUrl100").map(|u| large_artwork(&u));
    Some(candidate)
}

#[async_trait]
impl MetadataProvider for Itunes {
    fn id(&self) -> &'static str {
        ID
    }

    async fn search(&self, query: &Query) -> Vec<Candidate> {
        let term = query.text();
        let url = format!("{}/search", self.base);
        let Some(value) = get_json(
            &self.client,
            &self.limiter,
            ID,
            &url,
            &[("term", term.as_str()), ("entity", "song"), ("limit", "5")],
        )
        .await
        else {
            return Vec::new();
        };
        value
            .get("results")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(parse_item).take(5).collect())
            .unwrap_or_default()
    }
}

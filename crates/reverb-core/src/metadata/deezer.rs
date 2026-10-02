//! Provedor Deezer (sem chave). Usa a **busca simples** `q=<artista> <título>`: a forma avançada
//! combinada (`artist:"A" track:"T"`) devolve 0 resultados (E1, verificado em 2026-10-01).

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use super::provider::{get_json, year_of, Candidate, MetadataProvider, Query, RateLimiter};

pub const ID: &str = "deezer";

pub struct Deezer {
    client: reqwest::Client,
    base: String,
    limiter: Arc<RateLimiter>,
}

impl Deezer {
    /// 5 requisições por segundo.
    pub fn limiter() -> Arc<RateLimiter> {
        RateLimiter::per_second(5)
    }

    pub fn new(client: reqwest::Client, base: &str, limiter: Arc<RateLimiter>) -> Self {
        Self {
            client,
            base: base.trim_end_matches('/').to_string(),
            limiter,
        }
    }

    async fn get(&self, path: &str, query: &[(&str, &str)]) -> Option<Value> {
        let url = format!("{}/{path}", self.base);
        let value = get_json(&self.client, &self.limiter, ID, &url, query).await?;
        // A API devolve 200 com `{"error": {...}}` para ids inexistentes.
        if value.get("error").is_some() {
            tracing::warn!(provider = ID, path, "o Deezer devolveu um erro");
            return None;
        }
        Some(value)
    }

    /// Busca reversa por ISRC: `GET /track/isrc:{ISRC}`.
    pub async fn by_isrc(&self, isrc: &str) -> Option<Candidate> {
        let value = self.get(&format!("track/isrc:{isrc}"), &[]).await?;
        parse_track(&value)
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

/// Um item de `search` ou de `track/{id}` (os dois têm o mesmo formato básico).
pub(crate) fn parse_track(item: &Value) -> Option<Candidate> {
    let id = item.get("id").and_then(Value::as_u64)?;
    let title = text(item, "title")?;
    let mut candidate = Candidate::new(ID, id.to_string(), title);
    let artist = item.get("artist");
    if let Some(name) = artist.and_then(|a| text(a, "name")) {
        candidate.artists.push(name);
    }
    if let Some(album) = item.get("album") {
        candidate.album = text(album, "title");
        candidate.cover_url = text(album, "cover_xl").or_else(|| text(album, "cover_big"));
    }
    candidate.duration_s = item
        .get("duration")
        .and_then(Value::as_f64)
        .filter(|d| *d > 0.0);
    candidate.isrc = text(item, "isrc");
    candidate.track_no = number(item, "track_position");
    candidate.disc_no = number(item, "disk_number");
    candidate.year = text(item, "release_date").as_deref().and_then(year_of);
    Some(candidate)
}

#[async_trait]
impl MetadataProvider for Deezer {
    fn id(&self) -> &'static str {
        ID
    }

    async fn search(&self, query: &Query) -> Vec<Candidate> {
        let text = query.text();
        let Some(value) = self
            .get("search", &[("q", text.as_str()), ("limit", "5")])
            .await
        else {
            return Vec::new();
        };
        value
            .get("data")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(parse_track).take(5).collect())
            .unwrap_or_default()
    }

    /// `/track/{id}` (ISRC, faixa, disco, data) e `/album/{id}` (gêneros, total de faixas).
    async fn details(&self, mut candidate: Candidate) -> Candidate {
        let Some(track) = self
            .get(&format!("track/{}", candidate.provider_id), &[])
            .await
        else {
            return candidate;
        };
        candidate.isrc = text(&track, "isrc").or(candidate.isrc);
        candidate.track_no = number(&track, "track_position").or(candidate.track_no);
        candidate.disc_no = number(&track, "disk_number").or(candidate.disc_no);
        candidate.year = text(&track, "release_date")
            .as_deref()
            .and_then(year_of)
            .or(candidate.year);
        let album_id = track
            .get("album")
            .and_then(|a| a.get("id"))
            .and_then(Value::as_u64);
        if let Some(album_id) = album_id {
            if let Some(album) = self.get(&format!("album/{album_id}"), &[]).await {
                candidate.genre = album
                    .get("genres")
                    .and_then(|g| g.get("data"))
                    .and_then(Value::as_array)
                    .and_then(|genres| genres.first())
                    .and_then(|g| text(g, "name"))
                    .or(candidate.genre);
                candidate.track_total = number(&album, "nb_tracks").or(candidate.track_total);
                candidate.album_artist = album
                    .get("artist")
                    .and_then(|a| text(a, "name"))
                    .or(candidate.album_artist);
                candidate.year = text(&album, "release_date")
                    .as_deref()
                    .and_then(year_of)
                    .or(candidate.year);
            }
        }
        candidate
    }
}

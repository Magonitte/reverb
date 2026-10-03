//! LRCLIB é a única fonte de letras (F09 / E6). O texto LRC permanece intacto.

use std::sync::Arc;

use serde::Deserialize;

use crate::metadata::provider::{http_client, HTTP_TIMEOUT};
use crate::metadata::RateLimiter;
use crate::{CoreError, CoreResult};

pub const ENDPOINT: &str = "https://lrclib.net/api";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lyrics {
    pub plain: Option<String>,
    pub synced: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LyricsQuery {
    pub artist: String,
    pub title: String,
    pub album: Option<String>,
    pub duration_s: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    duration: f64,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

fn nonempty(text: Option<String>) -> Option<String> {
    // Verifica vazio, mas NÃO normaliza espaços, quebras ou timestamps.
    text.filter(|s| !s.trim().is_empty())
}

impl Record {
    fn delta(&self, query: &LyricsQuery) -> f64 {
        (self.duration - query.duration_s).abs()
    }

    fn lyrics(self) -> Option<Lyrics> {
        let lyrics = Lyrics {
            plain: nonempty(self.plain_lyrics),
            synced: nonempty(self.synced_lyrics),
        };
        (lyrics.plain.is_some() || lyrics.synced.is_some()).then_some(lyrics)
    }
}

pub struct LyricsClient {
    client: reqwest::Client,
    base: String,
    limiter: Arc<RateLimiter>,
}

impl Default for LyricsClient {
    fn default() -> Self {
        Self::new(http_client(), ENDPOINT, Self::limiter())
    }
}

impl LyricsClient {
    pub fn limiter() -> Arc<RateLimiter> {
        RateLimiter::per_second(5)
    }

    /// URL base incluindo `/api`; limitador compartilhado entre todos os jobs.
    pub fn new(client: reqwest::Client, base: &str, limiter: Arc<RateLimiter>) -> Self {
        Self {
            client,
            base: base.trim_end_matches('/').to_string(),
            limiter,
        }
    }

    /// `get` aceita Δ ≤ 2 s; fallback `search` escolhe a menor Δ ≤ 3 s.
    /// Ausência é `Ok(None)`; erros podem virar avisos no pipeline da tarefa 7.
    pub async fn fetch(&self, query: &LyricsQuery) -> CoreResult<Option<Lyrics>> {
        if query.artist.trim().is_empty()
            || query.title.trim().is_empty()
            || !query.duration_s.is_finite()
            || query.duration_s <= 0.0
        {
            return Ok(None);
        }
        let duration = query.duration_s.to_string();
        let params = [
            ("artist_name", query.artist.as_str()),
            ("track_name", query.title.as_str()),
            ("album_name", query.album.as_deref().unwrap_or("")),
            ("duration", duration.as_str()),
        ];
        // Mesmo uma falha de rede/JSON no get permite tentar search.
        let get = self.request("get", &params).await.and_then(|value| {
            value
                .map(serde_json::from_value::<Record>)
                .transpose()
                .map_err(CoreError::from)
        });
        let get_error = match get {
            Ok(Some(record)) if record.delta(query) <= 2.0 => {
                if let Some(lyrics) = record.lyrics() {
                    return Ok(Some(lyrics));
                }
                None
            }
            Err(error) => Some(error),
            _ => None,
        };
        if let Some(error) = &get_error {
            tracing::warn!(%error, "LRCLIB get falhou; tentando search");
        }
        let search = self.request("search", &params[..2]).await?;
        let Some(value) = search else {
            return match get_error {
                Some(error) => Err(error),
                None => Ok(None),
            };
        };
        let records: Vec<Record> = serde_json::from_value(value)?;
        let best = records
            .into_iter()
            .filter_map(|record| {
                let delta = record.delta(query);
                if delta <= 3.0 {
                    record.lyrics().map(|lyrics| (delta, lyrics))
                } else {
                    None
                }
            })
            .min_by(|(a, _), (b, _)| a.total_cmp(b));
        match best {
            Some((_, lyrics)) => Ok(Some(lyrics)),
            None => match get_error {
                Some(error) => Err(error),
                None => Ok(None),
            },
        }
    }

    async fn request(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> CoreResult<Option<serde_json::Value>> {
        self.limiter.acquire().await;
        let response = self
            .client
            .get(format!("{}/{path}", self.base))
            .timeout(HTTP_TIMEOUT)
            .query(params)
            .send()
            .await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(CoreError::coded(
                "lyrics_http",
                format!("LRCLIB HTTP {}", response.status()),
            ));
        }
        Ok(Some(response.json().await?))
    }
}

#[cfg(test)]
mod tests;

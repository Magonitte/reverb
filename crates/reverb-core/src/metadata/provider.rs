//! Contrato dos provedores de metadados, candidato, endpoints injetáveis e limitador de taxa
//! (arquitetura §11, "Provedores").

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::time::Instant;
use ts_rs::TS;

use super::normalize::norm;

/// `User-Agent` de todas as requisições (o MusicBrainz exige um identificável).
pub fn user_agent() -> String {
    format!(
        "Reverb/{} (+https://github.com/Magonitte/reverb)",
        env!("CARGO_PKG_VERSION")
    )
}

/// Prazo de cada requisição aos provedores.
pub const HTTP_TIMEOUT: Duration = Duration::from_secs(15);

pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(user_agent())
        .timeout(HTTP_TIMEOUT)
        .build()
        .expect("cliente HTTP dos provedores")
}

/// URLs base dos provedores (injetáveis para os testes com `wiremock`; sem barra final).
#[derive(Debug, Clone)]
pub struct Endpoints {
    pub deezer: String,
    pub itunes: String,
    pub musicbrainz: String,
    pub cover_art: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            deezer: "https://api.deezer.com".to_string(),
            itunes: "https://itunes.apple.com".to_string(),
            musicbrainz: "https://musicbrainz.org".to_string(),
            cover_art: "https://coverartarchive.org".to_string(),
        }
    }
}

impl Endpoints {
    /// Todos os provedores apontando para o mesmo servidor falso.
    pub fn all(base: &str) -> Self {
        let base = base.trim_end_matches('/').to_string();
        Self {
            deezer: format!("{base}/deezer"),
            itunes: format!("{base}/itunes"),
            musicbrainz: format!("{base}/musicbrainz"),
            cover_art: format!("{base}/coverart"),
        }
    }
}

/// Um possível casamento devolvido por um provedor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Candidate {
    pub provider: String,
    pub provider_id: String,
    pub title: String,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub track_no: Option<u32>,
    pub track_total: Option<u32>,
    pub disc_no: Option<u32>,
    pub duration_s: Option<f64>,
    pub cover_url: Option<String>,
    pub mb_recording_id: Option<String>,
    pub isrc: Option<String>,
}

impl Candidate {
    pub fn new(provider: &str, provider_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            provider: provider.to_string(),
            provider_id: provider_id.into(),
            title: title.into(),
            artists: Vec::new(),
            album: None,
            album_artist: None,
            year: None,
            genre: None,
            track_no: None,
            track_total: None,
            disc_no: None,
            duration_s: None,
            cover_url: None,
            mb_recording_id: None,
            isrc: None,
        }
    }
}

/// O que se procura.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub title: String,
    pub artists: Vec<String>,
    pub duration_s: Option<f64>,
}

impl Query {
    pub fn new(title: impl Into<String>, artists: Vec<String>, duration_s: Option<f64>) -> Self {
        Self {
            title: title.into(),
            artists,
            duration_s,
        }
    }

    /// Artista principal (o primeiro).
    pub fn artist(&self) -> Option<&str> {
        self.artists.first().map(String::as_str)
    }

    /// Texto da busca simples: `<artista> <título>`.
    pub fn text(&self) -> String {
        match self.artist() {
            Some(artist) => format!("{artist} {}", self.title),
            None => self.title.clone(),
        }
    }

    /// Chave de cache (§11, item 6): artista e título normalizados + duração arredondada.
    pub fn cache_part(&self) -> String {
        format!(
            "{}|{}|{}",
            norm(self.artist().unwrap_or_default()),
            norm(&self.title),
            self.duration_s.map_or(-1, |d| d.round() as i64)
        )
    }
}

#[async_trait]
pub trait MetadataProvider: Send + Sync {
    /// Identificador estável: `deezer`, `itunes`, `musicbrainz`.
    fn id(&self) -> &'static str;

    /// Até 5 candidatos. Falhas (HTTP ≠ 200, timeout, JSON inválido) viram lista vazia + log.
    async fn search(&self, query: &Query) -> Vec<Candidate>;

    /// Detalhes extras só para o candidato vencedor (faixa, gênero, ISRC…).
    async fn details(&self, candidate: Candidate) -> Candidate {
        candidate
    }
}

/// Espaça as chamadas de um provedor (`interval` mínimo entre o início de duas requisições).
/// Uma instância por provedor, compartilhada por todo o processo através do `MetadataService`.
#[derive(Debug)]
pub struct RateLimiter {
    interval: Duration,
    next: Mutex<Option<Instant>>,
}

impl RateLimiter {
    pub fn new(interval: Duration) -> Arc<Self> {
        Arc::new(Self {
            interval,
            next: Mutex::new(None),
        })
    }

    /// `n` requisições por segundo.
    pub fn per_second(n: u32) -> Arc<Self> {
        Self::new(Duration::from_secs_f64(1.0 / f64::from(n.max(1))))
    }

    pub async fn acquire(&self) {
        let mut next = self.next.lock().await;
        let now = Instant::now();
        let slot = next.map_or(now, |at| at.max(now));
        *next = Some(slot + self.interval);
        drop(next);
        if slot > now {
            tokio::time::sleep_until(slot).await;
        }
    }
}

/// GET que devolve o JSON, ou `None` (com log) em qualquer falha.
pub(crate) async fn get_json(
    client: &reqwest::Client,
    limiter: &RateLimiter,
    provider: &str,
    url: &str,
    query: &[(&str, &str)],
) -> Option<serde_json::Value> {
    limiter.acquire().await;
    let response = match client.get(url).query(query).send().await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(provider, error = %error.without_url(), "consulta de metadados falhou");
            return None;
        }
    };
    let status = response.status();
    if !status.is_success() {
        tracing::warn!(provider, %status, "provedor de metadados respondeu com erro");
        return None;
    }
    // O iTunes responde `text/javascript`: lê como texto e interpreta à mão.
    let body = match response.text().await {
        Ok(body) => body,
        Err(error) => {
            tracing::warn!(provider, error = %error.without_url(), "corpo da resposta ilegível");
            return None;
        }
    };
    match serde_json::from_str(&body) {
        Ok(value) => Some(value),
        Err(error) => {
            tracing::warn!(provider, %error, "resposta do provedor não é JSON");
            None
        }
    }
}

pub(crate) fn year_of(date: &str) -> Option<u32> {
    let digits: String = date.chars().take(4).collect();
    (digits.len() == 4)
        .then(|| digits.parse::<u32>().ok())
        .flatten()
        .filter(|y| (1000..=2999).contains(y))
}

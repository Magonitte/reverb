//! Cache de 30 dias (`metadata_cache`) que envolve cada provedor (arquitetura §11, item 6).

use std::sync::Arc;

use async_trait::async_trait;
use sha2::{Digest, Sha256};

use super::provider::{Candidate, MetadataProvider, Query};
use crate::db::Db;
use crate::error::CoreResult;

/// Validade de uma entrada.
pub const TTL_SECS: i64 = 30 * 24 * 60 * 60;

/// Relógio injetável (segundos Unix); os testes avançam o tempo sem esperar.
pub type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

pub fn system_clock() -> Clock {
    Arc::new(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64)
    })
}

fn key(parts: &str) -> String {
    hex::encode(Sha256::digest(parts.as_bytes()))
}

/// Chave do §11 item 6: `sha256(provider|artista normalizado|título normalizado|duração)`.
pub fn search_key(provider: &str, query: &Query) -> String {
    key(&format!("{provider}|{}", query.cache_part()))
}

pub fn details_key(provider: &str, provider_id: &str) -> String {
    key(&format!("{provider}|detail|{provider_id}"))
}

#[derive(Clone)]
pub struct Cache {
    db: Db,
    clock: Clock,
}

impl Cache {
    pub fn new(db: Db, clock: Clock) -> Self {
        Self { db, clock }
    }

    /// Valor ainda válido (criado há no máximo 30 dias).
    pub async fn get(&self, key: &str) -> CoreResult<Option<String>> {
        let key = key.to_string();
        let now = (self.clock)();
        self.db
            .call(move |conn| {
                let mut stmt = conn.prepare_cached(
                    "SELECT value, created_at FROM metadata_cache WHERE key = ?1",
                )?;
                let mut rows = stmt.query([&key])?;
                let Some(row) = rows.next()? else {
                    return Ok(None);
                };
                let value: String = row.get(0)?;
                let created_at: i64 = row.get(1)?;
                Ok((now - created_at <= TTL_SECS).then_some(value))
            })
            .await
    }

    pub async fn put(&self, key: &str, value: &str) -> CoreResult<()> {
        let (key, value) = (key.to_string(), value.to_string());
        let now = (self.clock)();
        self.db
            .call(move |conn| {
                conn.execute(
                    "INSERT INTO metadata_cache (key, value, created_at) VALUES (?1, ?2, ?3) \
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value, created_at = excluded.created_at",
                    rusqlite::params![key, value, now],
                )?;
                Ok(())
            })
            .await
    }
}

/// Provedor com cache na frente. Só guarda respostas **não vazias**: uma lista vazia pode ser
/// uma falha (HTTP 500, timeout) e não deve ficar presa por 30 dias.
pub struct CachedProvider {
    inner: Arc<dyn MetadataProvider>,
    cache: Cache,
}

impl CachedProvider {
    pub fn new(inner: Arc<dyn MetadataProvider>, cache: Cache) -> Self {
        Self { inner, cache }
    }
}

#[async_trait]
impl MetadataProvider for CachedProvider {
    fn id(&self) -> &'static str {
        self.inner.id()
    }

    async fn search(&self, query: &Query) -> Vec<Candidate> {
        let cache_key = search_key(self.id(), query);
        if let Ok(Some(text)) = self.cache.get(&cache_key).await {
            if let Ok(found) = serde_json::from_str::<Vec<Candidate>>(&text) {
                return found;
            }
        }
        let found = self.inner.search(query).await;
        if !found.is_empty() {
            if let Ok(text) = serde_json::to_string(&found) {
                if let Err(error) = self.cache.put(&cache_key, &text).await {
                    tracing::warn!(%error, "não foi possível gravar o cache de metadados");
                }
            }
        }
        found
    }

    async fn details(&self, candidate: Candidate) -> Candidate {
        let cache_key = details_key(self.id(), &candidate.provider_id);
        if let Ok(Some(text)) = self.cache.get(&cache_key).await {
            if let Ok(found) = serde_json::from_str::<Candidate>(&text) {
                return found;
            }
        }
        let detailed = self.inner.details(candidate.clone()).await;
        if detailed != candidate {
            if let Ok(text) = serde_json::to_string(&detailed) {
                if let Err(error) = self.cache.put(&cache_key, &text).await {
                    tracing::warn!(%error, "não foi possível gravar o cache de metadados");
                }
            }
        }
        detailed
    }
}

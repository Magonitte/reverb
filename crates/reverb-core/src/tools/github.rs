//! Cliente mínimo da API de releases do GitHub, com cache de 1 h (arquitetura §16).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::{CoreError, CoreResult};

pub const DEFAULT_API_BASE: &str = "https://api.github.com";
/// Limite anônimo da API: 60 requisições/h ⇒ reaproveitar respostas por 1 h.
pub const CACHE_TTL_SECS: u64 = 3600;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub zipball_url: Option<String>,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

impl Release {
    pub fn asset(&self, name: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.name == name)
    }
}

#[derive(Serialize, Deserialize)]
struct CachedRelease {
    fetched_at: u64,
    release: Release,
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub struct GithubClient {
    http: reqwest::Client,
    base_url: String,
    token: Option<String>,
    db: Option<Db>,
    memory: Mutex<HashMap<String, CachedRelease>>,
}

impl GithubClient {
    /// `base_url` é injetável (testes usam um servidor falso). `token` costuma vir de `GITHUB_TOKEN`.
    pub fn new(
        http: reqwest::Client,
        base_url: impl Into<String>,
        token: Option<String>,
        db: Option<Db>,
    ) -> Self {
        Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token: token.filter(|t| !t.is_empty()),
            db,
            memory: Mutex::new(HashMap::new()),
        }
    }

    fn kv_key(repo: &str) -> String {
        format!("github.latest.{repo}")
    }

    async fn cached(&self, repo: &str) -> Option<CachedRelease> {
        {
            let memory = self.memory.lock().expect("cache do GitHub");
            if let Some(hit) = memory.get(repo) {
                return Some(CachedRelease {
                    fetched_at: hit.fetched_at,
                    release: hit.release.clone(),
                });
            }
        }
        let db = self.db.as_ref()?;
        let raw = db.kv_get(&Self::kv_key(repo)).await.ok().flatten()?;
        let parsed: CachedRelease = serde_json::from_str(&raw).ok()?;
        self.memory.lock().expect("cache do GitHub").insert(
            repo.to_string(),
            CachedRelease {
                fetched_at: parsed.fetched_at,
                release: parsed.release.clone(),
            },
        );
        Some(parsed)
    }

    async fn store(&self, repo: &str, release: &Release) {
        let entry = CachedRelease {
            fetched_at: now_secs(),
            release: release.clone(),
        };
        if let (Some(db), Ok(json)) = (&self.db, serde_json::to_string(&entry)) {
            if let Err(e) = db.kv_set(&Self::kv_key(repo), &json).await {
                tracing::warn!(error = %e, "não foi possível gravar o cache de releases");
            }
        }
        self.memory
            .lock()
            .expect("cache do GitHub")
            .insert(repo.to_string(), entry);
    }

    /// Último release do repositório. Usa o cache (memória + `kv`) por 1 h, salvo `force`.
    pub async fn latest(&self, repo: &str, force: bool) -> CoreResult<Release> {
        if !force {
            if let Some(hit) = self.cached(repo).await {
                if now_secs().saturating_sub(hit.fetched_at) < CACHE_TTL_SECS {
                    return Ok(hit.release);
                }
            }
        }
        let url = format!("{}/repos/{repo}/releases/latest", self.base_url);
        let mut request = self
            .http
            .get(&url)
            .header("Accept", "application/vnd.github+json");
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let kind = if status == reqwest::StatusCode::FORBIDDEN
                || status == reqwest::StatusCode::TOO_MANY_REQUESTS
            {
                "github_rate_limit"
            } else {
                "github_http"
            };
            return Err(CoreError::coded(
                kind,
                format!("GitHub respondeu HTTP {status} para {repo}"),
            ));
        }
        let release: Release = response.json().await.map_err(|e| {
            CoreError::coded(
                "github_parse",
                format!("resposta inesperada do GitHub: {e}"),
            )
        })?;
        self.store(repo, &release).await;
        Ok(release)
    }
}

#[cfg(test)]
mod tests;

use super::{ImportedCollection, ImportedTrack};
use crate::metadata::{
    official::normalize_isrc,
    provider::{http_client, RateLimiter},
    MetadataFields,
};
use crate::{CoreError, CoreResult, SettingsService};
use serde_json::Value;
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio::time::Instant;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportUrl {
    pub provider: String,
    pub kind: String,
    pub id: String,
}
pub fn classify(input: &str) -> Option<ImportUrl> {
    let url = url::Url::parse(input.trim()).ok()?;
    if !matches!(url.scheme(), "https" | "http")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let host = url.host_str()?;
    let provider = match host {
        "deezer.com" | "www.deezer.com" => "deezer",
        "open.spotify.com" => "spotify",
        "link.deezer.com" => {
            return Some(ImportUrl {
                provider: "deezer".into(),
                kind: "short".into(),
                id: url.path().into(),
            })
        }
        _ => return None,
    };
    let parts: Vec<_> = url.path_segments()?.filter(|p| !p.is_empty()).collect();
    let start = if parts
        .first()
        .is_some_and(|p| p.len() == 2 || p.starts_with("intl-"))
    {
        1
    } else {
        0
    };
    if parts.len() != start + 2 {
        return None;
    }
    let kind = parts[start];
    let id = parts[start + 1];
    if !matches!(kind, "playlist" | "album" | "track" | "artist")
        || id.is_empty()
        || !id.bytes().all(|b| {
            if provider == "deezer" {
                b.is_ascii_digit()
            } else {
                b.is_ascii_alphanumeric()
            }
        })
    {
        return None;
    }
    Some(ImportUrl {
        provider: provider.into(),
        kind: kind.into(),
        id: id.into(),
    })
}

#[async_trait::async_trait]
pub trait ImportSource: Send + Sync {
    fn matches(&self, url: &str) -> bool;
    async fn fetch(&self, url: &str) -> CoreResult<ImportedCollection>;
}
fn text(v: &Value, key: &str) -> Option<String> {
    v[key].as_str().filter(|s| !s.is_empty()).map(str::to_owned)
}
fn identifier(v: &Value) -> CoreResult<String> {
    match &v["id"] {
        Value::String(s) if !s.is_empty() => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err(CoreError::coded("import_response", "Missing track ID")),
    }
}
pub fn deezer_track(v: &Value, album: Option<&Value>) -> CoreResult<ImportedTrack> {
    let a = album.unwrap_or(&v["album"]);
    let artists = v["contributors"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| text(v, "name")).collect::<Vec<_>>())
        .filter(|a| !a.is_empty())
        .unwrap_or_else(|| text(&v["artist"], "name").into_iter().collect());
    let fields = MetadataFields {
        title: text(v, "title")
            .ok_or_else(|| CoreError::coded("import_response", "Missing track title"))?,
        artist: Some(artists.join(", ")),
        artists,
        album: text(a, "title"),
        album_artist: text(&a["artist"], "name"),
        year: text(a, "release_date").and_then(|s| s.get(..4)?.parse().ok()),
        track_no: v["track_position"].as_u64().map(|n| n as u32),
        track_total: a["nb_tracks"].as_u64().map(|n| n as u32),
        disc_no: v["disk_number"].as_u64().map(|n| n as u32),
        cover_url: text(a, "cover_xl"),
        ..Default::default()
    };
    Ok(ImportedTrack {
        id: identifier(v)?,
        fields,
        duration_s: v["duration"].as_f64(),
        isrc: text(v, "isrc").and_then(|s| normalize_isrc(&s)),
        explicit: v["explicit_lyrics"].as_bool(),
    })
}
pub struct DeezerSource {
    client: reqwest::Client,
    pub base: String,
    limiter: Arc<RateLimiter>,
}
impl Default for DeezerSource {
    fn default() -> Self {
        Self::new("https://api.deezer.com")
    }
}
impl DeezerSource {
    pub fn new(base: &str) -> Self {
        Self {
            client: http_client(),
            base: base.trim_end_matches('/').into(),
            limiter: RateLimiter::new(Duration::from_millis(200)),
        }
    }
    pub async fn get(&self, path: &str) -> CoreResult<Value> {
        self.limiter.acquire().await;
        let u = if path.starts_with('/') {
            format!("{}{path}", self.base)
        } else {
            path.to_owned()
        };
        let parsed = url::Url::parse(&u)
            .map_err(|_| CoreError::coded("import_response", "Invalid pagination URL"))?;
        let base =
            url::Url::parse(&self.base).map_err(|_| CoreError::invalid("Invalid API endpoint"))?;
        if parsed.origin() != base.origin() {
            return Err(CoreError::coded(
                "import_response",
                "Untrusted pagination origin",
            ));
        }
        let response = self
            .client
            .get(u)
            .send()
            .await
            .map_err(|_| CoreError::coded("network", "Deezer request failed"))?;
        if !response.status().is_success() {
            return Err(CoreError::coded(
                "import_api",
                format!("HTTP {}", response.status().as_u16()),
            ));
        }
        let v: Value = response
            .json()
            .await
            .map_err(|_| CoreError::coded("import_response", "Invalid Deezer response"))?;
        if !v["error"].is_null() {
            return Err(CoreError::coded(
                if v["error"]["code"] == 800 {
                    "deezer_not_found"
                } else {
                    "import_api"
                },
                "Deezer collection unavailable",
            ));
        }
        Ok(v)
    }
    pub async fn pages(&self, path: &str) -> CoreResult<Vec<Value>> {
        let mut out = Vec::new();
        let mut next = Some(path.to_owned());
        let mut seen = HashSet::new();
        while let Some(u) = next {
            if !seen.insert(u.clone()) {
                return Err(CoreError::coded("import_response", "Pagination cycle"));
            }
            let page = self.get(&u).await?;
            out.extend(
                page["data"]
                    .as_array()
                    .ok_or_else(|| CoreError::coded("import_response", "Missing page data"))?
                    .iter()
                    .cloned(),
            );
            next = text(&page, "next");
        }
        Ok(out)
    }
    pub async fn resolve(&self, input: &str) -> CoreResult<ImportUrl> {
        let mut kind = classify(input).ok_or_else(|| CoreError::invalid("Invalid import URL"))?;
        if kind.kind == "short" {
            kind = self.resolve_short(input).await?;
        }
        Ok(kind)
    }
    async fn resolve_short(&self, input: &str) -> CoreResult<ImportUrl> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| CoreError::coded("network", "HTTP client failed"))?;
        let mut next = input.to_owned();
        for _ in 0..5 {
            let response = client
                .get(&next)
                .send()
                .await
                .map_err(|_| CoreError::coded("network", "Deezer short link failed"))?;
            let location = response
                .headers()
                .get("Location")
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| CoreError::coded("import_url", "Missing short link redirect"))?;
            let target = response
                .url()
                .join(location)
                .map_err(|_| CoreError::coded("import_url", "Invalid redirect URL"))?;
            let k = classify(target.as_str())
                .filter(|k| k.provider == "deezer")
                .ok_or_else(|| CoreError::coded("import_url", "Invalid short link target"))?;
            if k.kind != "short" {
                return Ok(k);
            }
            next = target.into();
        }
        Err(CoreError::coded("import_url", "Too many redirects"))
    }
}
#[async_trait::async_trait]
impl ImportSource for DeezerSource {
    fn matches(&self, u: &str) -> bool {
        classify(u).is_some_and(|k| k.provider == "deezer")
    }
    async fn fetch(&self, input: &str) -> CoreResult<ImportedCollection> {
        let k = self.resolve(input).await?;
        if !matches!(k.kind.as_str(), "playlist" | "album" | "track") {
            return Err(CoreError::invalid("A playlist, album or track is required"));
        }
        let meta = self.get(&format!("/{}/{}", k.kind, k.id)).await?;
        let data = if k.kind == "track" {
            vec![meta.clone()]
        } else {
            self.pages(&format!("/{}/{}/tracks?limit=100", k.kind, k.id))
                .await?
        };
        let mut tracks = Vec::new();
        for v in data {
            let v = if text(&v, "isrc").is_none() {
                self.get(&format!("/track/{}", identifier(&v)?)).await?
            } else {
                v
            };
            tracks.push(deezer_track(&v, (k.kind == "album").then_some(&meta))?);
        }
        Ok(ImportedCollection {
            provider: k.provider,
            id: k.id.clone(),
            url: format!("https://www.deezer.com/{}/{}", k.kind, k.id),
            title: text(&meta, "title")
                .ok_or_else(|| CoreError::coded("import_response", "Missing collection title"))?,
            cover: text(&meta, "picture_xl").or_else(|| text(&meta, "cover_xl")),
            tracks,
            checksum: text(&meta, "checksum"),
        })
    }
}

struct Token {
    value: String,
    expires: Instant,
    credentials: (String, String),
}
pub struct SpotifySource {
    client: reqwest::Client,
    settings: Arc<SettingsService>,
    base: String,
    accounts: String,
    token: Mutex<Option<Token>>,
    #[cfg(test)]
    retry_delays: Option<Arc<std::sync::Mutex<Vec<Duration>>>>,
}
impl SpotifySource {
    pub fn new(settings: Arc<SettingsService>) -> Self {
        Self::with_endpoints(
            settings,
            "https://api.spotify.com",
            "https://accounts.spotify.com",
        )
    }
    pub fn with_endpoints(settings: Arc<SettingsService>, base: &str, accounts: &str) -> Self {
        Self {
            client: http_client(),
            settings,
            base: base.into(),
            accounts: accounts.into(),
            token: Mutex::new(None),
            #[cfg(test)]
            retry_delays: None,
        }
    }
    async fn wait_retry(&self, delay: Duration) {
        #[cfg(test)]
        if let Some(delays) = &self.retry_delays {
            delays.lock().unwrap().push(delay);
            return;
        }
        tokio::time::sleep(delay).await;
    }
    async fn token(&self) -> CoreResult<String> {
        let s = self.settings.get();
        let credentials = (s.spotify_client_id, s.spotify_client_secret);
        if credentials.0.is_empty() || credentials.1.is_empty() {
            return Err(CoreError::coded(
                "spotify_credentials_missing",
                "Spotify credentials missing",
            ));
        }
        let mut cache = self.token.lock().await;
        if let Some(t) = cache
            .as_ref()
            .filter(|t| t.credentials == credentials && t.expires > Instant::now())
        {
            return Ok(t.value.clone());
        }
        let response = self
            .client
            .post(format!("{}/api/token", self.accounts))
            .basic_auth(&credentials.0, Some(&credentials.1))
            .header("content-type", "application/x-www-form-urlencoded")
            .body("grant_type=client_credentials")
            .send()
            .await
            .map_err(|_| CoreError::coded("network", "Spotify token request failed"))?;
        if !response.status().is_success() {
            return Err(CoreError::coded(
                "spotify_auth",
                "Spotify authentication failed",
            ));
        }
        let v: Value = response
            .json()
            .await
            .map_err(|_| CoreError::coded("import_response", "Invalid token response"))?;
        let value = text(&v, "access_token")
            .ok_or_else(|| CoreError::coded("import_response", "Missing access token"))?;
        crate::logging::register_secret(&value);
        *cache = Some(Token {
            value: value.clone(),
            credentials,
            expires: Instant::now()
                + Duration::from_secs(v["expires_in"].as_u64().unwrap_or(3600).saturating_sub(30)),
        });
        Ok(value)
    }
    async fn get(&self, path: &str, playlist: bool) -> CoreResult<Value> {
        let u = if path.starts_with('/') {
            format!("{}{path}", self.base)
        } else {
            path.into()
        };
        if url::Url::parse(&u).ok().map(|u| u.origin())
            != url::Url::parse(&self.base).ok().map(|u| u.origin())
        {
            return Err(CoreError::coded(
                "import_response",
                "Untrusted Spotify pagination",
            ));
        }
        for attempt in 0..4 {
            let response = self
                .client
                .get(&u)
                .bearer_auth(self.token().await?)
                .send()
                .await
                .map_err(|_| CoreError::coded("network", "Spotify request failed"))?;
            match response.status().as_u16() {
                200 => {
                    return response.json().await.map_err(|_| {
                        CoreError::coded("import_response", "Invalid Spotify response")
                    })
                }
                401 if attempt < 3 => {
                    *self.token.lock().await = None;
                }
                429 if attempt < 3 => {
                    let delay = response
                        .headers()
                        .get("Retry-After")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(1);
                    self.wait_retry(Duration::from_secs(delay)).await;
                }
                500..=599 if attempt < 3 => {
                    self.wait_retry(Duration::from_secs(1 << attempt)).await
                }
                403 | 404 if playlist => {
                    return Err(CoreError::coded(
                        "spotify_playlist_restricted",
                        "Spotify playlist access restricted",
                    ))
                }
                404 => {
                    return Err(CoreError::coded(
                        "spotify_not_found",
                        "Spotify collection not found",
                    ))
                }
                401 | 403 => return Err(CoreError::coded("spotify_auth", "Spotify access denied")),
                n => return Err(CoreError::coded("import_api", format!("HTTP {n}"))),
            }
        }
        Err(CoreError::coded("import_api", "Spotify retry limit"))
    }
    async fn pages(&self, path: &str, playlist: bool) -> CoreResult<Vec<Value>> {
        let mut next = Some(path.to_owned());
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        while let Some(u) = next {
            if !seen.insert(u.clone()) {
                return Err(CoreError::coded("import_response", "Pagination cycle"));
            }
            let v = self.get(&u, playlist).await?;
            out.extend(
                v["items"]
                    .as_array()
                    .ok_or_else(|| CoreError::coded("import_response", "Missing Spotify items"))?
                    .iter()
                    .cloned(),
            );
            next = text(&v, "next");
        }
        Ok(out)
    }
}
pub fn spotify_track(v: &Value, album: Option<&Value>) -> CoreResult<ImportedTrack> {
    let a = album.unwrap_or(&v["album"]);
    let artists = v["artists"]
        .as_array()
        .ok_or_else(|| CoreError::coded("import_response", "Missing artists"))?
        .iter()
        .filter_map(|v| text(v, "name"))
        .collect::<Vec<_>>();
    Ok(ImportedTrack {
        id: identifier(v)?,
        fields: MetadataFields {
            title: text(v, "name")
                .ok_or_else(|| CoreError::coded("import_response", "Missing title"))?,
            artist: Some(artists.join(", ")),
            artists,
            album: text(a, "name"),
            album_artist: a["artists"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| text(v, "name")),
            year: text(a, "release_date").and_then(|s| s.get(..4)?.parse().ok()),
            track_no: v["track_number"].as_u64().map(|n| n as u32),
            disc_no: v["disc_number"].as_u64().map(|n| n as u32),
            track_total: a["total_tracks"].as_u64().map(|n| n as u32),
            cover_url: a["images"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| text(v, "url")),
            ..Default::default()
        },
        duration_s: v["duration_ms"].as_f64().map(|n| n / 1000.),
        isrc: text(v, "external_ids")
            .or_else(|| text(&v["external_ids"], "isrc"))
            .and_then(|s| normalize_isrc(&s)),
        explicit: v["explicit"].as_bool(),
    })
}
#[async_trait::async_trait]
impl ImportSource for SpotifySource {
    fn matches(&self, u: &str) -> bool {
        classify(u).is_some_and(|k| k.provider == "spotify" && k.kind != "artist")
    }
    async fn fetch(&self, input: &str) -> CoreResult<ImportedCollection> {
        let k = classify(input)
            .filter(|k| self.matches(input) && k.kind != "artist")
            .ok_or_else(|| CoreError::invalid("Invalid Spotify import URL"))?;
        let plural = format!("{}s", k.kind);
        let meta = self
            .get(&format!("/v1/{plural}/{}", k.id), k.kind == "playlist")
            .await?;
        let mut tracks = Vec::new();
        if k.kind == "track" {
            tracks.push(spotify_track(&meta, None)?);
        } else {
            let items_endpoint = if k.kind == "playlist" {
                "items"
            } else {
                "tracks"
            };
            for v in self
                .pages(
                    &format!("/v1/{plural}/{}/{items_endpoint}?limit=50", k.id),
                    k.kind == "playlist",
                )
                .await?
            {
                let raw = if k.kind == "playlist" {
                    if v["track"].is_null() {
                        &v["item"]
                    } else {
                        &v["track"]
                    }
                } else {
                    &v
                };
                if raw.is_null() || raw["is_local"] == true {
                    continue;
                }
                let detailed = if raw["external_ids"]["isrc"].is_null() {
                    self.get(&format!("/v1/tracks/{}", identifier(raw)?), false)
                        .await?
                } else {
                    raw.clone()
                };
                tracks.push(spotify_track(
                    &detailed,
                    (k.kind == "album").then_some(&meta),
                )?);
            }
        }
        Ok(ImportedCollection {
            provider: k.provider,
            id: k.id.clone(),
            url: format!("https://open.spotify.com/{}/{}", k.kind, k.id),
            title: text(&meta, "name")
                .ok_or_else(|| CoreError::coded("import_response", "Missing collection name"))?,
            cover: meta["images"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| text(v, "url")),
            tracks,
            checksum: None,
        })
    }
}

//! Optional keyed providers. Secrets and tokens stay in Rust memory.
use crate::metadata::provider::{http_client, Candidate, Query};
use crate::{CoreError, CoreResult, SettingsService};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

struct Token {
    credentials: (String, String),
    value: String,
    expires: Instant,
}

pub struct KeyedProviders {
    client: reqwest::Client,
    token: Mutex<Option<Token>>,
    settings: Arc<SettingsService>,
    accounts: String,
    spotify: String,
    discogs: String,
}
impl KeyedProviders {
    pub fn new(settings: Arc<SettingsService>) -> Self {
        Self::with_endpoints(
            settings,
            "https://accounts.spotify.com",
            "https://api.spotify.com",
            "https://api.discogs.com",
        )
    }
    pub fn with_endpoints(
        settings: Arc<SettingsService>,
        accounts: &str,
        spotify: &str,
        discogs: &str,
    ) -> Self {
        Self {
            client: http_client(),
            token: Mutex::new(None),
            settings,
            accounts: accounts.into(),
            spotify: spotify.into(),
            discogs: discogs.into(),
        }
    }
    async fn json(request: reqwest::RequestBuilder) -> CoreResult<Value> {
        let response = request
            .send()
            .await
            .map_err(|_| CoreError::coded("network", "Provider request failed"))?;
        if !response.status().is_success() {
            return Err(CoreError::coded(
                "provider_api",
                format!("HTTP {}", response.status().as_u16()),
            ));
        }
        response
            .json()
            .await
            .map_err(|_| CoreError::coded("provider_api", "Invalid provider response"))
    }
    async fn spotify_token(&self) -> CoreResult<String> {
        let s = self.settings.get();
        if s.spotify_client_id.is_empty() || s.spotify_client_secret.is_empty() {
            return Err(CoreError::coded(
                "provider_key",
                "Spotify credentials missing",
            ));
        }
        let credentials = (s.spotify_client_id, s.spotify_client_secret);
        let mut cache = self.token.lock().await;
        if let Some(token) = cache
            .as_ref()
            .filter(|t| t.credentials == credentials && t.expires > Instant::now())
        {
            return Ok(token.value.clone());
        }
        let data = Self::json(
            self.client
                .post(format!("{}/api/token", self.accounts))
                .basic_auth(&credentials.0, Some(&credentials.1))
                .header("content-type", "application/x-www-form-urlencoded")
                .body("grant_type=client_credentials"),
        )
        .await?;
        let value = data["access_token"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| CoreError::coded("provider_api", "Missing access token"))?
            .to_string();
        crate::logging::register_secret(&value);
        let ttl = data["expires_in"]
            .as_u64()
            .unwrap_or(3600)
            .saturating_sub(30);
        *cache = Some(Token {
            credentials,
            value: value.clone(),
            expires: Instant::now() + Duration::from_secs(ttl),
        });
        Ok(value)
    }
    pub async fn spotify_search(&self, query: &Query) -> CoreResult<Vec<Candidate>> {
        let token = self.spotify_token().await?;
        let q = format!(
            "track:{} artist:{}",
            query.title,
            query.artist().unwrap_or_default()
        );
        let data = Self::json(
            self.client
                .get(format!("{}/v1/search", self.spotify))
                .bearer_auth(token)
                .query(&[("type", "track"), ("limit", "5"), ("q", q.as_str())]),
        )
        .await?;
        Ok(data["tracks"]["items"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|track| {
                let mut c =
                    Candidate::new("spotify", track["id"].as_str()?, track["name"].as_str()?);
                c.artists = track["artists"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|a| a["name"].as_str().map(str::to_string))
                    .collect();
                c.album = track["album"]["name"].as_str().map(str::to_string);
                c.album_artist = track["album"]["artists"][0]["name"]
                    .as_str()
                    .map(str::to_string);
                c.year = track["album"]["release_date"]
                    .as_str()
                    .and_then(|s| s.get(..4)?.parse().ok());
                c.cover_url = track["album"]["images"][0]["url"]
                    .as_str()
                    .map(str::to_string);
                c.duration_s = track["duration_ms"].as_f64().map(|v| v / 1000.0);
                c.isrc = track["external_ids"]["isrc"].as_str().map(str::to_string);
                c.track_no = track["track_number"].as_u64().map(|v| v as u32);
                c.disc_no = track["disc_number"].as_u64().map(|v| v as u32);
                Some(c)
            })
            .collect())
    }
    pub async fn discogs_search(&self, query: &Query) -> CoreResult<Vec<Candidate>> {
        let token = self.settings.get().discogs_token;
        if token.is_empty() {
            return Err(CoreError::coded("provider_key", "Discogs token missing"));
        }
        let data = Self::json(
            self.client
                .get(format!("{}/database/search", self.discogs))
                .header("Authorization", format!("Discogs token={token}"))
                .query(&[
                    ("type", "release"),
                    ("track", query.title.as_str()),
                    ("artist", query.artist().unwrap_or_default()),
                    ("per_page", "5"),
                ]),
        )
        .await?;
        let mut out = Vec::new();
        for release in data["results"].as_array().into_iter().flatten().take(5) {
            let Some(id) = release["id"].as_u64() else {
                continue;
            };
            let details = Self::json(
                self.client
                    .get(format!("{}/releases/{id}", self.discogs))
                    .header("Authorization", format!("Discogs token={token}")),
            )
            .await?;
            for track in details["tracklist"].as_array().into_iter().flatten() {
                let Some(title) = track["title"].as_str() else {
                    continue;
                };
                if crate::metadata::normalize::norm_plain(title)
                    != crate::metadata::normalize::norm_plain(&query.title)
                {
                    continue;
                }
                let mut c = Candidate::new("discogs", id.to_string(), title);
                c.artists = details["artists"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|a| a["name"].as_str().map(str::to_string))
                    .collect();
                if c.artists.is_empty() {
                    c.artists = query.artists.clone();
                }
                c.album = details["title"].as_str().map(str::to_string);
                c.year = details["year"].as_u64().map(|v| v as u32);
                c.genre = details["styles"][0]
                    .as_str()
                    .or(details["genres"][0].as_str())
                    .map(str::to_string);
                c.cover_url = details["images"][0]["uri"]
                    .as_str()
                    .or(release["cover_image"].as_str())
                    .map(str::to_string);
                c.duration_s = track["duration"].as_str().and_then(|s| {
                    let (m, sec) = s.split_once(':')?;
                    Some(m.parse::<f64>().ok()? * 60.0 + sec.parse::<f64>().ok()?)
                });
                out.push(c);
            }
        }
        Ok(out)
    }
    pub async fn test(&self, provider: &str) -> CoreResult<()> {
        let query = Query::new(
            "Never Gonna Give You Up",
            vec!["Rick Astley".into()],
            Some(214.0),
        );
        match provider {
            "spotify" => {
                self.spotify_search(&query).await?;
            }
            "discogs" => {
                self.discogs_search(&query).await?;
            }
            _ => return Err(CoreError::invalid("Unknown provider")),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    async fn setup() -> (MockServer, KeyedProviders) {
        let server = MockServer::start().await;
        let db = crate::Db::open_in_memory().unwrap();
        let settings = Arc::new(
            SettingsService::new(db, Arc::new(crate::MemorySink::new()))
                .await
                .unwrap(),
        );
        settings.update(serde_json::from_value(serde_json::json!({"spotifyClientId":"id","spotifyClientSecret":"secret","discogsToken":"discogs"})).unwrap()).await.unwrap();
        let api =
            KeyedProviders::with_endpoints(settings, &server.uri(), &server.uri(), &server.uri());
        (server, api)
    }
    #[tokio::test]
    async fn spotify_reuses_token_until_expiration_and_maps_track() {
        let (server, api) = setup().await;
        Mock::given(method("POST"))
            .and(path("/api/token"))
            .and(header("Authorization", "Basic aWQ6c2VjcmV0"))
            .and(body_string("grant_type=client_credentials"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"access_token":"test-token","expires_in":3600}),
                ),
            )
            .expect(2)
            .mount(&server)
            .await;
        Mock::given(method("GET")).and(path("/v1/search")).and(header("Authorization","Bearer test-token")).respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"tracks":{"items":[{"id":"track","name":"Song","artists":[{"name":"Artist"}],"album":{"name":"Album","release_date":"1987-01-01","images":[{"url":"https://cover"}]},"duration_ms":214000,"external_ids":{"isrc":"GBARL9300135"}}]}}))).mount(&server).await;
        let q = Query::new("Song", vec!["Artist".into()], Some(214.0));
        assert_eq!(api.spotify_search(&q).await.unwrap()[0].year, Some(1987));
        api.spotify_search(&q).await.unwrap();
        api.token.lock().await.as_mut().unwrap().expires = Instant::now() - Duration::from_secs(1);
        api.spotify_search(&q).await.unwrap();
        assert!(!crate::logging::redact("Bearer test-token").contains("test-token"));
    }
    #[tokio::test]
    async fn spotify_reports_forbidden_without_exposing_credentials() {
        let (server, api) = setup().await;
        Mock::given(path("/api/token"))
            .respond_with(ResponseTemplate::new(403).set_body_string("secret id"))
            .mount(&server)
            .await;
        let error = api.test("spotify").await.unwrap_err();
        assert_eq!(error.kind(), "provider_api");
        assert_eq!(error.to_string(), "HTTP 403");
    }
    #[tokio::test]
    async fn discogs_fetches_release_track_and_style() {
        let (server, api) = setup().await;
        Mock::given(path("/database/search"))
            .and(header("Authorization", "Discogs token=discogs"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"results":[{"id":7}]})),
            )
            .mount(&server)
            .await;
        Mock::given(path("/releases/7")).respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"title":"Album","year":1987,"styles":["Synth-pop"],"artists":[{"name":"Artist"}],"tracklist":[{"title":"Song","duration":"3:34"}],"images":[{"uri":"https://cover"}]}))).mount(&server).await;
        let c = api
            .discogs_search(&Query::new("Song", vec!["Artist".into()], None))
            .await
            .unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].genre.as_deref(), Some("Synth-pop"));
        assert_eq!(c[0].duration_s, Some(214.0));
    }
}

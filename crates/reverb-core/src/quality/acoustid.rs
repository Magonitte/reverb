use crate::metadata::{Candidate, ScoredCandidate};
use crate::{CoreError, CoreResult};
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

static LIMITER: std::sync::LazyLock<std::sync::Arc<crate::metadata::provider::RateLimiter>> =
    std::sync::LazyLock::new(|| crate::metadata::provider::RateLimiter::per_second(3));

pub async fn lookup(
    fpcalc: &Path,
    audio: &Path,
    key: &str,
    endpoint: &str,
) -> CoreResult<Vec<ScoredCandidate>> {
    let output = crate::tools::run_capture(
        fpcalc,
        ["-json".to_string(), audio.to_string_lossy().into_owned()],
        |_| {},
        Duration::from_secs(120),
    )
    .await
    .map_err(|_| CoreError::coded("fingerprint", "fpcalc failed"))?;
    if !output.success {
        return Err(CoreError::coded("fingerprint", "fpcalc failed"));
    }
    let data: Value = serde_json::from_str(&output.stdout)?;
    let fingerprint = data["fingerprint"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| CoreError::coded("fingerprint", "Empty fingerprint"))?;
    let duration = data["duration"]
        .as_f64()
        .filter(|d| d.is_finite() && *d > 0.0)
        .ok_or_else(|| CoreError::coded("fingerprint", "Invalid fingerprint duration"))?;
    LIMITER.acquire().await;
    let response = crate::metadata::provider::http_client()
        .get(endpoint)
        .query(&[
            ("client", key),
            ("meta", "recordings+releasegroups"),
            ("duration", &duration.round().to_string()),
            ("fingerprint", fingerprint),
        ])
        .send()
        .await
        .map_err(|_| CoreError::coded("network", "AcoustID request failed"))?;
    if !response.status().is_success() {
        return Err(CoreError::coded(
            "provider_api",
            format!("HTTP {}", response.status().as_u16()),
        ));
    }
    let body: Value = response
        .json()
        .await
        .map_err(|_| CoreError::coded("provider_api", "Invalid AcoustID response"))?;
    parse(&body)
}
pub fn parse(body: &Value) -> CoreResult<Vec<ScoredCandidate>> {
    if body["status"] != "ok" {
        return Err(CoreError::coded("provider_api", "AcoustID lookup failed"));
    }
    let mut out = Vec::new();
    for result in body["results"].as_array().into_iter().flatten() {
        let Some(score) = result["score"]
            .as_f64()
            .filter(|s| s.is_finite() && (0.8..=1.0).contains(s))
        else {
            continue;
        };
        let Some(id) = result["id"].as_str() else {
            continue;
        };
        for recording in result["recordings"].as_array().into_iter().flatten() {
            let (Some(mb), Some(title)) = (recording["id"].as_str(), recording["title"].as_str())
            else {
                continue;
            };
            let mut c = Candidate::new("acoustid", id, title);
            c.mb_recording_id = Some(mb.into());
            c.duration_s = recording["duration"].as_f64();
            c.artists = recording["artists"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|a| a["name"].as_str().map(str::to_string))
                .collect();
            c.album = recording["releasegroups"][0]["title"]
                .as_str()
                .map(str::to_string);
            out.push(ScoredCandidate {
                score: score * 0.95,
                candidate: c,
            });
        }
    }
    out.sort_by(|a, b| b.score.total_cmp(&a.score));
    out.truncate(5);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn confidence_is_acoustic_and_keeps_both_ids() {
        let c=parse(&serde_json::json!({"status":"ok","results":[{"id":"sound","score":0.9,"recordings":[{"id":"mb","title":"Song","artists":[{"name":"Artist"}]}]},{"id":"bad","score":0.79}]})).unwrap();
        assert_eq!(c.len(), 1);
        assert!((c[0].score - 0.855).abs() < 1e-6);
        assert_eq!(c[0].candidate.provider_id, "sound");
        assert_eq!(c[0].candidate.mb_recording_id.as_deref(), Some("mb"));
    }
}

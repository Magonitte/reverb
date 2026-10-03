//! Parsers of public provider responses. No purchase, email or login flow is automated.
use super::{error, SourceTrack};
use crate::ytdlp::{DownloadError, SearchResult};
use serde_json::Value;
use url::Url;

fn value(text: &str) -> Result<Value, DownloadError> {
    serde_json::from_str(text).map_err(|e| {
        error(&format!(
            "Invalid source response at line {} column {}",
            e.line(),
            e.column()
        ))
    })
}
fn text(v: &Value, key: &str) -> Option<String> {
    v[key].as_str().map(str::to_string)
}
fn scalar(v: &Value) -> Option<String> {
    v.as_str().map(str::to_string).or_else(|| {
        v.as_array()
            .and_then(|a| a.first())
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}
fn year(v: &Value) -> Option<u32> {
    v.as_str()
        .and_then(|s| s.get(..4))
        .and_then(|s| s.parse().ok())
}
fn number(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

pub fn archive_search(body: &str, base: &str) -> Result<Vec<SearchResult>, DownloadError> {
    let v = value(body)?;
    let docs = v["response"]["docs"]
        .as_array()
        .ok_or_else(|| error("Invalid Archive search response"))?;
    Ok(docs
        .iter()
        .filter_map(|item| {
            let id = text(item, "identifier")?;
            Some(SearchResult {
                title: text(item, "title").unwrap_or_else(|| id.clone()),
                url: Some(format!("{base}/details/{id}")),
                id,
                duration: None,
                channel: scalar(&item["creator"]),
            })
        })
        .collect())
}

pub fn archive(body: &str, id: &str, base: &str) -> Result<Vec<SourceTrack>, DownloadError> {
    let v = value(body)?;
    if v["is_dark"].as_bool() == Some(true)
        || v["is_restricted"].as_bool() == Some(true)
        || v["metadata"]["access-restricted-item"].as_str() == Some("true")
    {
        return Err(error("Restricted Archive item"));
    }
    let files = v["files"]
        .as_array()
        .ok_or_else(|| error("Invalid Archive metadata"))?;
    let rank = |file: &Value| match file["format"].as_str().unwrap_or("") {
        "24bit Flac" => 3,
        "Flac" => 2,
        "VBR MP3" => 1,
        _ => 0,
    };
    let best = files.iter().map(rank).max().unwrap_or(0);
    if best == 0 {
        return Err(error("No downloadable audio in Archive item"));
    }
    let mut tracks = Vec::new();
    for file in files
        .iter()
        .filter(|f| rank(f) == best && f["private"].as_str() != Some("true"))
    {
        let Some(name) = text(file, "name") else {
            continue;
        };
        let mut download = Url::parse(base).map_err(|_| error("Invalid Archive endpoint"))?;
        download
            .path_segments_mut()
            .map_err(|_| error("Invalid Archive endpoint"))?
            .extend(["download", id, &name]);
        let mut page = Url::parse(&format!("{base}/details/{id}"))
            .map_err(|_| error("Invalid Archive endpoint"))?;
        page.query_pairs_mut().append_pair("file", &name);
        tracks.push(SourceTrack {
            id: format!("{id}/{name}"),
            title: text(file, "title").unwrap_or(name),
            artist: scalar(&v["metadata"]["creator"]),
            album: scalar(&v["metadata"]["title"]),
            year: year(&v["metadata"]["date"]),
            track_no: number(&file["track"]).map(|n| n as u32),
            url: page.into(),
            download_url: download.into(),
            ext: if best >= 2 { "flac" } else { "mp3" }.into(),
            duration: number(&file["length"]),
        });
    }
    tracks.sort_by_key(|t| t.track_no.unwrap_or(u32::MAX));
    Ok(tracks)
}

pub fn jamendo(body: &str) -> Result<Vec<SourceTrack>, DownloadError> {
    let v = value(body)?;
    if v["headers"]["status"].as_str() != Some("success") {
        return Err(error("Jamendo API rejected the request"));
    }
    let results = v["results"]
        .as_array()
        .ok_or_else(|| error("Invalid Jamendo response"))?;
    Ok(results
        .iter()
        .filter(|v| v["audiodownload_allowed"].as_bool() == Some(true))
        .filter_map(|v| {
            let id = text(v, "id")?;
            let download_url = text(v, "audiodownload").filter(|s| !s.is_empty())?;
            Some(SourceTrack {
                id: id.clone(),
                title: text(v, "name")?,
                artist: text(v, "artist_name"),
                album: text(v, "album_name"),
                year: year(&v["releasedate"]),
                track_no: number(&v["position"]).map(|n| n as u32),
                url: format!("https://www.jamendo.com/track/{id}"),
                download_url,
                ext: "flac".into(),
                duration: number(&v["duration"]),
            })
        })
        .collect())
}

fn html_decode(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(index) = rest.find('&') {
        out.push_str(&rest[..index]);
        rest = &rest[index..];
        let Some(end) = rest.find(';').filter(|n| *n < 16) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        let decoded = match entity {
            "quot" => Some('"'),
            "amp" => Some('&'),
            "apos" | "#39" => Some('\''),
            "lt" => Some('<'),
            "gt" => Some('>'),
            _ => entity
                .strip_prefix("#x")
                .and_then(|v| u32::from_str_radix(v, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|v| v.parse().ok()))
                .and_then(char::from_u32),
        };
        if let Some(c) = decoded {
            out.push(c);
        } else {
            out.push_str(&rest[..=end]);
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

pub fn attribute_json(html: &str, attribute: &str) -> Result<Value, DownloadError> {
    let regex = regex::Regex::new(&format!(
        r#"{}\s*=\s*(?:"([^"]*)"|'([^']*)')"#,
        regex::escape(attribute)
    ))
    .map_err(|_| error("Invalid HTML attribute"))?;
    let found = regex
        .captures(html)
        .ok_or_else(|| error("Bandcamp page structure changed"))?;
    let raw = found
        .get(1)
        .or_else(|| found.get(2))
        .ok_or_else(|| error("Bandcamp page structure changed"))?;
    value(&html_decode(raw.as_str()))
}

pub fn bandcamp_free_page(album: &Value) -> Result<String, DownloadError> {
    if album["current"]["require_email"].as_bool() == Some(true)
        || album["require_email"].as_bool() == Some(true)
    {
        return Err(error("errors.bandcampRestricted"));
    }
    let page = text(album, "freeDownloadPage")
        .filter(|p| !p.is_empty())
        .ok_or_else(|| error("errors.bandcampRestricted"))?;
    let url = Url::parse(&page).map_err(|_| error("Invalid Bandcamp free download page"))?;
    if url.scheme() != "https"
        || url.host_str() != Some("bandcamp.com")
        || url.path() != "/download"
    {
        return Err(error("Invalid Bandcamp free download page"));
    }
    Ok(page)
}

pub fn bandcamp_status(body: &str) -> Result<Value, DownloadError> {
    let body = body.trim();
    let body = body
        .strip_prefix("var _statDL_result")
        .and_then(|s| s.split_once('='))
        .map_or(body, |(_, v)| v);
    let legacy = regex::Regex::new(r"^\s*\{\s*result\s*:\s*'(ok|pending|err)'\s*\}\s*;?\s*$")
        .map_err(|_| error("Invalid Bandcamp status parser"))?;
    if let Some(found) = legacy.captures(body) {
        return Ok(serde_json::json!({"result":&found[1]}));
    }
    let start = body
        .find('{')
        .ok_or_else(|| error("Invalid Bandcamp download status"))?;
    serde_json::Deserializer::from_str(&body[start..])
        .into_iter::<Value>()
        .next()
        .ok_or_else(|| error("Missing Bandcamp download status"))?
        .map_err(|e| {
            error(&format!(
                "Invalid Bandcamp status at {}:{} structure {}",
                e.line(),
                e.column(),
                body[start..]
                    .chars()
                    .take(45)
                    .map(|c| if c.is_alphanumeric() { 'x' } else { c })
                    .collect::<String>()
            ))
        })
}

pub fn bandcamp(
    album: &Value,
    blob: &Value,
    page: &str,
) -> Result<Vec<SourceTrack>, DownloadError> {
    let items = blob["download_items"]
        .as_array()
        .ok_or_else(|| error("Bandcamp download page changed"))?;
    let item = items
        .iter()
        .find(|v| v["payment_type"].as_str() == Some("free"))
        .ok_or_else(|| error("errors.bandcampRestricted"))?;
    let download = text(&item["downloads"]["flac"], "url")
        .ok_or_else(|| error("Bandcamp FLAC unavailable"))?;
    let url = Url::parse(&download).map_err(|_| error("Invalid Bandcamp download URL"))?;
    if url.scheme() != "https" || !url.host_str().is_some_and(|h| h.ends_with(".bandcamp.com")) {
        return Err(error("Invalid Bandcamp download URL"));
    }
    let tracks = album["trackinfo"]
        .as_array()
        .ok_or_else(|| error("Bandcamp track list changed"))?;
    let base = Url::parse(page).map_err(|_| error("Invalid Bandcamp page"))?;
    Ok(tracks
        .iter()
        .filter(|t| t["is_downloadable"].as_bool() == Some(true))
        .filter_map(|t| {
            let title = text(t, "title")?;
            let track_url = base.join(t["title_link"].as_str()?).ok()?.to_string();
            Some(SourceTrack {
                id: t["track_id"].as_u64()?.to_string(),
                title,
                artist: text(album, "artist"),
                album: text(&album["current"], "title"),
                year: None,
                track_no: number(&t["track_num"]).map(|n| n as u32),
                url: track_url,
                download_url: download.clone(),
                ext: "flac".into(),
                duration: number(&t["duration"]),
            })
        })
        .collect())
}

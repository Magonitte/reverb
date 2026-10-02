//! Modelos do JSON do yt-dlp (`-J`): vídeo, coleção e resultado de busca (arquitetura §8).

use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

/// `null` e ausente viram o valor padrão.
fn null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Thumbnail {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Chapter {
    pub title: String,
    pub start_time: f64,
    pub end_time: f64,
}

#[derive(Deserialize)]
struct RawChapter {
    title: Option<String>,
    start_time: f64,
    end_time: f64,
}

/// Formato só de áudio (sem vídeo e com codec de áudio conhecido).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AudioFormat {
    pub format_id: String,
    pub acodec: String,
    pub abr: Option<f64>,
    pub ext: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub duration: Option<f64>,
    pub channel: Option<String>,
    pub uploader: Option<String>,
    pub thumbnail: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
    pub categories: Vec<String>,
    pub track: Option<String>,
    pub artist: Option<String>,
    pub artists: Vec<String>,
    pub creators: Vec<String>,
    pub album: Option<String>,
    pub release_year: Option<u32>,
    pub release_date: Option<String>,
    pub chapters: Vec<Chapter>,
    pub audio_formats: Vec<AudioFormat>,
    /// Maior `abr` entre os formatos de áudio; `None` se nenhum informar.
    pub best_audio_abr: Option<f64>,
    /// `track` presente e artista conhecido: metadado oficial do YouTube Music.
    pub is_official_track: bool,
    pub webpage_url: Option<String>,
    pub extractor_key: Option<String>,
}

#[derive(Deserialize)]
struct RawFormat {
    format_id: Option<String>,
    acodec: Option<String>,
    vcodec: Option<String>,
    abr: Option<f64>,
    ext: Option<String>,
}

#[derive(Deserialize)]
struct RawVideo {
    id: String,
    #[serde(default)]
    title: Option<String>,
    duration: Option<f64>,
    channel: Option<String>,
    uploader: Option<String>,
    thumbnail: Option<String>,
    #[serde(default, deserialize_with = "null_default")]
    thumbnails: Vec<Thumbnail>,
    #[serde(default, deserialize_with = "null_default")]
    categories: Vec<String>,
    track: Option<String>,
    artist: Option<String>,
    #[serde(default, deserialize_with = "null_default")]
    artists: Vec<String>,
    #[serde(default, deserialize_with = "null_default")]
    creators: Vec<String>,
    album: Option<String>,
    release_year: Option<u32>,
    release_date: Option<String>,
    #[serde(default, deserialize_with = "null_default")]
    chapters: Vec<RawChapter>,
    #[serde(default, deserialize_with = "null_default")]
    formats: Vec<RawFormat>,
    webpage_url: Option<String>,
    extractor_key: Option<String>,
}

fn non_empty(text: Option<String>) -> Option<String> {
    text.filter(|t| !t.trim().is_empty())
}

fn audio_format(raw: RawFormat) -> Option<AudioFormat> {
    let has_no_video = raw.vcodec.as_deref() == Some("none");
    let acodec = raw.acodec.filter(|codec| codec != "none")?;
    has_no_video.then_some(())?;
    Some(AudioFormat {
        format_id: raw.format_id?,
        acodec,
        abr: raw.abr,
        ext: raw.ext.unwrap_or_default(),
    })
}

impl VideoInfo {
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        let raw: RawVideo = serde_json::from_str(text)?;
        let audio_formats: Vec<AudioFormat> =
            raw.formats.into_iter().filter_map(audio_format).collect();
        let best_audio_abr = audio_formats
            .iter()
            .filter_map(|f| f.abr)
            .fold(None, |best: Option<f64>, abr| {
                Some(best.map_or(abr, |b| b.max(abr)))
            });
        let track = non_empty(raw.track);
        let artist = non_empty(raw.artist);
        let is_official_track = track.is_some() && (!raw.artists.is_empty() || artist.is_some());
        Ok(Self {
            title: raw.title.unwrap_or_else(|| raw.id.clone()),
            id: raw.id,
            duration: raw.duration,
            channel: raw.channel,
            uploader: raw.uploader,
            thumbnail: raw.thumbnail,
            thumbnails: raw.thumbnails,
            categories: raw.categories,
            track,
            artist,
            artists: raw.artists,
            creators: raw.creators,
            album: non_empty(raw.album),
            release_year: raw.release_year,
            release_date: raw.release_date,
            chapters: raw
                .chapters
                .into_iter()
                .map(|c| Chapter {
                    title: c.title.unwrap_or_default(),
                    start_time: c.start_time,
                    end_time: c.end_time,
                })
                .collect(),
            audio_formats,
            best_audio_abr,
            is_official_track,
            webpage_url: raw.webpage_url,
            extractor_key: raw.extractor_key,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CollectionEntry {
    pub id: String,
    pub title: Option<String>,
    pub duration: Option<f64>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CollectionInfo {
    pub id: Option<String>,
    pub title: Option<String>,
    pub channel: Option<String>,
    pub thumbnail: Option<String>,
    pub entries: Vec<CollectionEntry>,
}

#[derive(Deserialize)]
struct RawEntry {
    id: Option<String>,
    title: Option<String>,
    duration: Option<f64>,
    url: Option<String>,
}

#[derive(Deserialize)]
struct RawCollection {
    id: Option<String>,
    title: Option<String>,
    channel: Option<String>,
    uploader: Option<String>,
    thumbnail: Option<String>,
    #[serde(default, deserialize_with = "null_default")]
    thumbnails: Vec<Thumbnail>,
    #[serde(default, deserialize_with = "null_default")]
    entries: Vec<Option<RawEntry>>,
}

impl CollectionInfo {
    /// Entradas nulas ou sem `id` (itens removidos/privados) são descartadas.
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        let raw: RawCollection = serde_json::from_str(text)?;
        let entries = raw
            .entries
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                Some(CollectionEntry {
                    id: entry.id?,
                    title: entry.title,
                    duration: entry.duration,
                    url: entry.url,
                })
            })
            .collect();
        Ok(Self {
            id: raw.id,
            title: raw.title,
            channel: raw.channel.or(raw.uploader),
            thumbnail: raw
                .thumbnail
                .or_else(|| raw.thumbnails.last().map(|image| image.url.clone())),
            entries,
        })
    }
}

/// Item de busca (modo `--flat-playlist`: a duração só existe no `ytsearch`, não no YouTube Music).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub url: Option<String>,
    pub duration: Option<f64>,
    pub channel: Option<String>,
}

#[derive(Deserialize)]
struct RawSearchEntry {
    id: Option<String>,
    title: Option<String>,
    url: Option<String>,
    duration: Option<f64>,
    channel: Option<String>,
    uploader: Option<String>,
}

#[derive(Deserialize)]
struct RawSearch {
    #[serde(default, deserialize_with = "null_default")]
    entries: Vec<Option<RawSearchEntry>>,
}

impl SearchResult {
    pub fn list_from_json(text: &str) -> Result<Vec<Self>, serde_json::Error> {
        let raw: RawSearch = serde_json::from_str(text)?;
        Ok(raw
            .entries
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let id = entry.id?;
                Some(Self {
                    title: entry.title.unwrap_or_else(|| id.clone()),
                    id,
                    url: entry.url,
                    duration: entry.duration,
                    channel: entry.channel.or(entry.uploader),
                })
            })
            .collect())
    }
}

#[cfg(test)]
mod tests;

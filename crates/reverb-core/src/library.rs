//! Repositório da biblioteca (§5 / F09). Funções síncronas para usar dentro de `Db::call`.

use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::metadata::official::normalize_isrc;
use crate::metadata::{ContentType, ScoredCandidate};
use crate::queue::Job;
use crate::tagging::TrackTags;
use crate::transcode::ProbeInfo;
use crate::{CoreError, CoreResult};

pub mod files;
pub mod query;
pub mod watch;
pub use query::{
    albums, artists, clear, delete, dismiss, list, LibraryDateRange, LibraryPage, LibraryQuery,
    LibrarySort,
};

/// Espelho da tabela `library`; inclui os campos reservados para importação e reexame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LibraryItem {
    #[ts(type = "number")]
    pub id: i64,
    pub file_path: String,
    pub missing: bool,
    pub provider: Option<String>,
    pub source_id: Option<String>,
    pub source_url: Option<String>,
    pub isrc: Option<String>,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_no: Option<u32>,
    pub track_total: Option<u32>,
    pub disc_no: Option<u32>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub duration_s: Option<f64>,
    pub codec: Option<String>,
    pub bitrate_kbps: Option<f64>,
    pub source_abr_kbps: Option<f64>,
    pub profile_id: Option<String>,
    pub content_type: ContentType,
    pub metadata_source: Option<String>,
    pub confidence: Option<f64>,
    pub needs_review: bool,
    pub review_candidates: Option<Vec<ScoredCandidate>>,
    pub has_lyrics: bool,
    pub has_synced_lyrics: bool,
    pub cover_source: Option<String>,
    pub acoustid_id: Option<String>,
    pub mb_recording_id: Option<String>,
    pub replaygain_db: Option<f64>,
    pub lossless_verdict: Option<String>,
    pub origin: String,
    #[ts(type = "number")]
    pub added_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

/// Resultado final do arquivo, separado da identificação e das informações de origem do job.
/// O chamador só insere depois de gravar as tags e mover o áudio com sucesso.
#[derive(Debug, Clone)]
pub struct DownloadedFile {
    pub file_path: String,
    pub tags: TrackTags,
    pub probe: Option<ProbeInfo>,
    pub source_abr_kbps: Option<f64>,
    pub content_type: ContentType,
    pub has_synced_lyrics: bool,
    pub cover_source: Option<String>,
    pub replaygain_db: Option<f64>,
}

fn from_row(row: &Row<'_>) -> CoreResult<LibraryItem> {
    let content_type: String = row.get("content_type")?;
    let candidates: Option<String> = row.get("review_candidates_json")?;
    Ok(LibraryItem {
        id: row.get("id")?,
        file_path: row.get("file_path")?,
        missing: row.get("missing")?,
        provider: row.get("provider")?,
        source_id: row.get("source_id")?,
        source_url: row.get("source_url")?,
        isrc: row.get("isrc")?,
        title: row.get("title")?,
        artist: row.get("artist")?,
        album: row.get("album")?,
        album_artist: row.get("album_artist")?,
        track_no: row.get("track_no")?,
        track_total: row.get("track_total")?,
        disc_no: row.get("disc_no")?,
        year: row.get("year")?,
        genre: row.get("genre")?,
        duration_s: row.get("duration_s")?,
        codec: row.get("codec")?,
        bitrate_kbps: row.get("bitrate_kbps")?,
        source_abr_kbps: row.get("source_abr_kbps")?,
        profile_id: row.get("profile_id")?,
        content_type: match content_type.as_str() {
            "music" => ContentType::Music,
            "other" => ContentType::Other,
            _ => {
                return Err(CoreError::Internal(format!(
                    "valor inválido em library.content_type: {content_type}"
                )))
            }
        },
        metadata_source: row.get("metadata_source")?,
        confidence: row.get("confidence")?,
        needs_review: row.get("needs_review")?,
        review_candidates: candidates.map(|s| serde_json::from_str(&s)).transpose()?,
        has_lyrics: row.get("has_lyrics")?,
        has_synced_lyrics: row.get("has_synced_lyrics")?,
        cover_source: row.get("cover_source")?,
        acoustid_id: row.get("acoustid_id")?,
        mb_recording_id: row.get("mb_recording_id")?,
        replaygain_db: row.get("replaygain_db")?,
        lossless_verdict: row.get("lossless_verdict")?,
        origin: row.get("origin")?,
        added_at: row.get("added_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// Insere uma nova linha; um caminho já registrado é erro, nunca sobrescreve o registro anterior.
/// Não muda o job: o passo final do pipeline fará a associação `job.library_id` na tarefa 7.
pub fn insert_from_job(conn: &Connection, job: &Job, file: &DownloadedFile) -> CoreResult<i64> {
    if file.file_path.trim().is_empty()
        || file.file_path.contains('\0')
        || file.tags.title.trim().is_empty()
    {
        return Err(CoreError::invalid(
            "Biblioteca exige caminho e título não vazios",
        ));
    }
    let metadata = job.metadata_result.as_ref();
    let content_type = metadata
        .map(|m| m.content_type)
        .unwrap_or(file.content_type);
    let confidence = (content_type == ContentType::Music)
        .then(|| metadata.map(|m| m.confidence).or(job.confidence))
        .flatten();
    let duration = file.probe.as_ref().map(|p| p.duration_s).or(job.duration_s);
    if [
        confidence,
        duration,
        file.source_abr_kbps,
        file.replaygain_db,
    ]
    .into_iter()
    .flatten()
    .any(|n| !n.is_finite())
        || confidence.is_some_and(|n| !(0.0..=1.0).contains(&n))
        || duration.is_some_and(|n| n < 0.0)
        || file.source_abr_kbps.is_some_and(|n| n < 0.0)
    {
        return Err(CoreError::invalid(
            "Dados numéricos inválidos para a biblioteca",
        ));
    }
    let has_lyrics = file
        .tags
        .lyrics
        .as_ref()
        .is_some_and(|s| !s.trim().is_empty());
    if file.has_synced_lyrics && !has_lyrics {
        return Err(CoreError::invalid(
            "Letra sincronizada exige texto de letra",
        ));
    }
    let isrc = metadata
        .and_then(|m| m.isrc.as_deref())
        .and_then(normalize_isrc)
        .or_else(|| file.tags.isrc.as_deref().and_then(normalize_isrc));
    let needs_review = metadata.is_some_and(|m| m.needs_review());
    if metadata.filter(|_| needs_review).is_some_and(|m| {
        m.candidates.iter().any(|c| {
            !c.score.is_finite()
                || !(0.0..=1.0).contains(&c.score)
                || c.candidate
                    .duration_s
                    .is_some_and(|d| !d.is_finite() || d < 0.0)
        })
    }) {
        return Err(CoreError::invalid("Candidato de revisão inválido"));
    }
    let candidates = metadata
        .filter(|_| needs_review)
        .map(|m| serde_json::to_string(&m.candidates))
        .transpose()?;
    let now = crate::queue::repo::now();
    conn.execute(
        "INSERT INTO library (file_path, provider, source_id, source_url, isrc, title, artist, album, album_artist, \
         track_no, track_total, disc_no, year, genre, duration_s, codec, bitrate_kbps, source_abr_kbps, profile_id, \
         content_type, metadata_source, confidence, needs_review, review_candidates_json, has_lyrics, has_synced_lyrics, \
         cover_source, mb_recording_id, replaygain_db, origin, added_at, updated_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,'download',?30,?30)",
        params![file.file_path, job.provider, job.source_id, job.source_url, isrc, file.tags.title,
            file.tags.artist, file.tags.album, file.tags.album_artist, file.tags.track_no, file.tags.track_total,
            file.tags.disc_no, file.tags.year, file.tags.genre, duration, file.probe.as_ref().map(|p| &p.codec),
            file.probe.as_ref().and_then(|p| p.bitrate_kbps).map(f64::from), file.source_abr_kbps, job.profile_id,
            content_type.as_str(), metadata.map(|m| &m.source),
            confidence, needs_review, candidates, has_lyrics, file.has_synced_lyrics, file.cover_source,
            metadata.and_then(|m| m.fields.mb_recording_id.as_deref()), file.replaygain_db, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> CoreResult<Option<LibraryItem>> {
    let mut stmt = conn.prepare_cached("SELECT * FROM library WHERE id = ?1")?;
    let mut rows = stmt.query([id])?;
    rows.next()?.map(from_row).transpose()
}

/// Mesma identidade usada pela fila. Inclui arquivos marcados como ausentes, como na F04.
pub fn find_by_source(
    conn: &Connection,
    provider: &str,
    source_id: &str,
    profile_id: &str,
) -> CoreResult<Option<LibraryItem>> {
    let mut stmt = conn.prepare_cached("SELECT * FROM library WHERE provider=?1 AND source_id=?2 AND profile_id=?3 ORDER BY id LIMIT 1")?;
    let mut rows = stmt.query(params![provider, source_id, profile_id])?;
    rows.next()?.map(from_row).transpose()
}

/// Uma gravação pode existir em vários perfis/fontes. Devolve todos em ordem estável de id.
pub fn find_by_isrc(conn: &Connection, isrc: &str) -> CoreResult<Vec<LibraryItem>> {
    let Some(isrc) = normalize_isrc(isrc) else {
        return Ok(Vec::new());
    };
    let mut stmt = conn.prepare_cached("SELECT * FROM library WHERE isrc=?1 ORDER BY id")?;
    let mut rows = stmt.query([isrc])?;
    let mut items = Vec::new();
    while let Some(row) = rows.next()? {
        items.push(from_row(row)?);
    }
    Ok(items)
}

/// JPEG de 256 px no máximo para a Atividade, sempre derivado da capa embutida.
pub fn cover_thumbnail(path: &std::path::Path) -> CoreResult<Option<String>> {
    cover_thumbnail_with_size(path, 256)
}

pub fn cover_thumbnail_with_size(path: &std::path::Path, size: u32) -> CoreResult<Option<String>> {
    if !(1..=1200).contains(&size) {
        return Err(CoreError::invalid("cover size must be between 1 and 1200"));
    }
    use base64::Engine;
    use image::codecs::jpeg::JpegEncoder;
    let Some(cover) = crate::tagging::read_tags(path)?.cover else {
        return Ok(None);
    };
    let processed = crate::artwork::process(&cover.data)?;
    let image = image::load_from_memory(&processed)
        .map_err(|e| CoreError::coded("artwork_decode", e.to_string()))?
        .resize(size, size, image::imageops::FilterType::Lanczos3);
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, 85)
        .encode_image(&image)
        .map_err(|e| CoreError::coded("artwork_decode", e.to_string()))?;
    Ok(Some(format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    )))
}

#[cfg(test)]
mod tests;

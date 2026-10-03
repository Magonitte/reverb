//! Biblioteca persistente e busca FTS5 (F10). Todos os filtros usam parâmetros SQL.

use rusqlite::{params_from_iter, types::Value, Connection};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{from_row, get, LibraryItem};
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LibrarySort {
    #[default]
    AddedDesc,
    Title,
    Artist,
    Album,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LibraryDateRange {
    Today,
    Week,
    Month,
    #[default]
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct LibraryQuery {
    pub text: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub format: Option<String>,
    pub date_range: LibraryDateRange,
    pub needs_review: Option<bool>,
    pub missing: Option<bool>,
    pub sort: LibrarySort,
    pub offset: u32,
    pub limit: u32,
}

impl Default for LibraryQuery {
    fn default() -> Self {
        Self {
            text: None,
            artist: None,
            album: None,
            format: None,
            date_range: LibraryDateRange::All,
            needs_review: None,
            missing: None,
            sort: LibrarySort::AddedDesc,
            offset: 0,
            limit: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LibraryPage {
    pub items: Vec<LibraryItem>,
    #[ts(type = "number")]
    pub total: i64,
    pub offset: u32,
    pub limit: u32,
}

const COLUMNS: &str = "*";

/// Treat input as words, never as FTS syntax; each term is an ANDed prefix.
pub fn search_expression(text: &str) -> Option<String> {
    let words: Vec<_> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| format!("\"{s}\"*"))
        .collect();
    (!words.is_empty()).then(|| words.join(" AND "))
}

pub fn list(conn: &Connection, query: &LibraryQuery) -> CoreResult<LibraryPage> {
    if query.limit == 0 || query.limit > 500 {
        return Err(CoreError::invalid(
            "library limit must be between 1 and 500",
        ));
    }
    let mut clauses = vec!["1=1".to_string()];
    let mut values: Vec<Value> = vec![];
    if let Some(text) = query.text.as_ref().filter(|s| !s.trim().is_empty()) {
        match search_expression(text) {
            Some(expr) => {
                clauses
                    .push("id IN (SELECT rowid FROM library_fts WHERE library_fts MATCH ?)".into());
                values.push(expr.into());
            }
            None => clauses.push("0=1".into()),
        }
    }
    for (column, value) in [("artist", &query.artist), ("album", &query.album)] {
        if let Some(value) = value {
            clauses.push(format!("{column} = ?"));
            values.push(value.clone().into());
        }
    }
    if let Some(format) = &query.format {
        let format = format.to_ascii_lowercase();
        if !["mp3", "m4a", "opus", "ogg", "flac", "wav"].contains(&format.as_str()) {
            return Err(CoreError::invalid("unsupported library format"));
        }
        clauses.push("lower(file_path) LIKE ?".into());
        values.push(format!("%.{format}").into());
    }
    for (column, flag) in [
        ("needs_review", query.needs_review),
        ("missing", query.missing),
    ] {
        if let Some(flag) = flag {
            clauses.push(format!("{column} = ?"));
            values.push(i64::from(flag).into());
        }
    }
    let days = match query.date_range {
        LibraryDateRange::All => None,
        LibraryDateRange::Today => Some(0),
        LibraryDateRange::Week => Some(6),
        LibraryDateRange::Month => Some(29),
    };
    if let Some(days) = days {
        clauses.push("added_at >= unixepoch('now','localtime','start of day',?,'utc')".into());
        values.push(format!("-{days} days").into());
    }
    let where_sql = clauses.join(" AND ");
    // Both queries share the same snapshot (callers serialize access through Db).
    let total = conn.query_row(
        &format!("SELECT count(*) FROM library WHERE {where_sql}"),
        params_from_iter(&values),
        |r| r.get(0),
    )?;
    let order = match query.sort {
        LibrarySort::AddedDesc => "added_at DESC,id DESC",
        LibrarySort::Title => "title COLLATE NOCASE,id",
        LibrarySort::Artist => "artist COLLATE NOCASE,title COLLATE NOCASE,id",
        LibrarySort::Album => "album COLLATE NOCASE,disc_no,track_no,title COLLATE NOCASE,id",
    };
    values.push(i64::from(query.limit).into());
    values.push(i64::from(query.offset).into());
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM library WHERE {where_sql} ORDER BY {order} LIMIT ? OFFSET ?"
    ))?;
    let mut rows = stmt.query(params_from_iter(&values))?;
    let mut items = Vec::new();
    while let Some(row) = rows.next()? {
        items.push(from_row(row)?);
    }
    Ok(LibraryPage {
        items,
        total,
        offset: query.offset,
        limit: query.limit,
    })
}

pub fn artists(conn: &Connection) -> CoreResult<Vec<String>> {
    distinct(conn, "artist", None)
}
pub fn albums(conn: &Connection, artist: Option<&str>) -> CoreResult<Vec<String>> {
    distinct(conn, "album", artist)
}

fn distinct(conn: &Connection, column: &str, artist: Option<&str>) -> CoreResult<Vec<String>> {
    let filter = if artist.is_some() {
        " AND artist=?"
    } else {
        ""
    };
    let mut stmt = conn.prepare(&format!("SELECT DISTINCT {column} FROM library WHERE {column} IS NOT NULL AND trim({column}) <> ''{filter} ORDER BY {column} COLLATE NOCASE,{column}"))?;
    let items = stmt
        .query_map(params_from_iter(artist), |r| r.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

/// Records are removed only after trashing succeeds. Missing files can still be removed.
/// Earlier successes remain committed if a later file cannot be trashed.
pub fn delete(conn: &mut Connection, ids: &[i64], delete_files: bool) -> CoreResult<usize> {
    let mut removed = 0;
    for &id in ids {
        let Some(item) = get(conn, id)? else {
            continue;
        };
        if delete_files {
            let audio = std::path::PathBuf::from(&item.file_path);
            let mut files = vec![audio.clone(), audio.with_extension("lrc")];
            files.retain(|p| p.exists());
            if !files.is_empty() {
                trash::delete_all(&files).map_err(|e| CoreError::coded("disk", e.to_string()))?;
            }
        }
        removed += conn.execute("DELETE FROM library WHERE id=?", [id])?;
    }
    Ok(removed)
}

pub fn clear(conn: &Connection) -> CoreResult<usize> {
    Ok(conn.execute("DELETE FROM library", [])?)
}

pub fn dismiss(conn: &Connection, id: i64) -> CoreResult<()> {
    if conn.execute(
        "UPDATE library SET needs_review=0,updated_at=unixepoch() WHERE id=?",
        [id],
    )? == 0
    {
        return Err(CoreError::coded("not_found", "library item not found"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;

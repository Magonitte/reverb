use super::model::{Sync, SyncItem, SyncResult};
use crate::{CoreError, CoreResult};
use rusqlite::{params, Connection, OptionalExtension, Row};

const COLUMNS: &str = "s.id,s.provider,s.url,s.playlist_id,s.title,s.thumbnail,s.profile_id,s.output_dir,s.interval_hours,s.max_items,s.remove_deleted,s.write_m3u,s.enabled,s.last_sync_at,s.last_result_json,s.created_at,(SELECT count(*) FROM sync_items i WHERE i.sync_id=s.id AND i.state='present')";

fn row(row: &Row<'_>) -> rusqlite::Result<Sync> {
    let json: Option<String> = row.get(14)?;
    let last_result = json
        .map(|json| serde_json::from_str(&json))
        .transpose()
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(14, rusqlite::types::Type::Text, Box::new(e))
        })?;
    Ok(Sync {
        id: row.get(0)?,
        provider: row.get(1)?,
        url: row.get(2)?,
        playlist_id: row.get(3)?,
        title: row.get(4)?,
        thumbnail: row.get(5)?,
        profile_id: row.get(6)?,
        output_dir: row.get(7)?,
        interval_hours: row.get(8)?,
        max_items: row.get(9)?,
        remove_deleted: row.get(10)?,
        write_m3u: row.get(11)?,
        enabled: row.get(12)?,
        last_sync_at: row.get(13)?,
        last_result,
        created_at: row.get(15)?,
        item_count: row.get(16)?,
    })
}
pub fn list(conn: &Connection) -> CoreResult<Vec<Sync>> {
    Ok(conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM syncs s ORDER BY s.created_at,s.id"
        ))?
        .query_map([], row)?
        .collect::<Result<_, _>>()?)
}
pub fn get(conn: &Connection, id: &str) -> CoreResult<Sync> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM syncs s WHERE s.id=?"),
        [id],
        row,
    )
    .optional()?
    .ok_or_else(|| CoreError::coded("not_found", "Playlist sync not found"))
}

pub fn items(conn: &Connection, id: &str) -> CoreResult<Vec<SyncItem>> {
    let mut stmt=conn.prepare("SELECT i.sync_id,i.source_id,i.position,i.title,i.state,i.job_id,l.id,j.status,l.file_path,COALESCE(l.missing,0) FROM sync_items i LEFT JOIN jobs j ON j.id=i.job_id LEFT JOIN library l ON l.id=COALESCE(i.library_id,j.library_id) WHERE i.sync_id=? ORDER BY i.position,i.source_id")?;
    let items = stmt
        .query_map([id], |row| {
            Ok(SyncItem {
                sync_id: row.get(0)?,
                source_id: row.get(1)?,
                position: row.get(2)?,
                title: row.get(3)?,
                state: row.get(4)?,
                job_id: row.get(5)?,
                library_id: row.get(6)?,
                job_status: row.get(7)?,
                file_path: row.get(8)?,
                missing: row.get(9)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(items)
}
pub fn save_result(conn: &Connection, id: &str, result: &SyncResult, now: i64) -> CoreResult<()> {
    conn.execute(
        "UPDATE syncs SET last_sync_at=?,last_result_json=? WHERE id=?",
        params![now, serde_json::to_string(result)?, id],
    )?;
    Ok(())
}
pub fn library_match(
    conn: &Connection,
    provider: &str,
    source: &str,
    profile: &str,
) -> CoreResult<Option<i64>> {
    let mut stmt=conn.prepare("SELECT id,file_path FROM library WHERE provider=? AND source_id=? AND profile_id=? AND missing=0 ORDER BY id")?;
    for row in stmt.query_map(params![provider, source, profile], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })? {
        let (id, path) = row?;
        if std::path::Path::new(&path).is_file() {
            return Ok(Some(id));
        }
    }
    Ok(None)
}
pub fn ensure_changed(changed: usize) -> CoreResult<()> {
    if changed == 0 {
        Err(CoreError::coded("not_found", "Playlist sync not found"))
    } else {
        Ok(())
    }
}

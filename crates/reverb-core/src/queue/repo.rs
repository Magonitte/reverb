//! Acesso à tabela `jobs` (funções síncronas sobre a conexão; o chamador usa `Db::call`).

use std::collections::HashSet;

use rusqlite::{params, Connection, Row};

use super::model::{DuplicateHit, Job, JobOptions, JobStage, JobStatus, MoveTarget, PlaylistCtx};
use crate::error::{CoreError, CoreResult};

const COLUMNS: &str = "id, kind, provider, source_url, source_id, title, artist, thumbnail, \
    duration_s, profile_id, options_json, metadata_override_json, metadata_result_json, confidence, \
    warnings_json, playlist_ctx_json, sync_id, status, stage, progress, overall_progress, speed_bps, eta_s, error_kind, \
    error_message, attempts, output_path, library_id, position, created_at, updated_at, finished_at";

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

fn bad(column: &str, value: &str) -> CoreError {
    CoreError::Internal(format!("valor inválido em jobs.{column}: {value}"))
}

fn from_row(row: &Row<'_>) -> CoreResult<Job> {
    let options_json: String = row.get("options_json")?;
    let override_json: Option<String> = row.get("metadata_override_json")?;
    let result_json: Option<String> = row.get("metadata_result_json")?;
    let warnings_json: String = row.get("warnings_json")?;
    let ctx_json: Option<String> = row.get("playlist_ctx_json")?;
    let status: String = row.get("status")?;
    let stage: String = row.get("stage")?;
    Ok(Job {
        id: row.get("id")?,
        kind: row.get("kind")?,
        provider: row.get("provider")?,
        source_url: row.get("source_url")?,
        source_id: row.get("source_id")?,
        title: row.get("title")?,
        artist: row.get("artist")?,
        thumbnail: row.get("thumbnail")?,
        duration_s: row.get("duration_s")?,
        profile_id: row.get("profile_id")?,
        options: serde_json::from_str::<JobOptions>(&options_json)?,
        metadata_override: override_json
            .map(|text| serde_json::from_str(&text))
            .transpose()?,
        confidence: row.get("confidence")?,
        metadata_result: result_json
            .map(|text| serde_json::from_str(&text))
            .transpose()?,
        warnings: serde_json::from_str(&warnings_json)?,
        playlist_ctx: ctx_json
            .map(|text| serde_json::from_str::<PlaylistCtx>(&text))
            .transpose()?,
        sync_id: row.get("sync_id")?,
        status: JobStatus::parse(&status).ok_or_else(|| bad("status", &status))?,
        stage: JobStage::parse(&stage).ok_or_else(|| bad("stage", &stage))?,
        progress: row.get("progress")?,
        overall_progress: row.get("overall_progress")?,
        speed_bps: row.get("speed_bps")?,
        eta_s: row.get("eta_s")?,
        error_kind: row.get("error_kind")?,
        error_message: row.get("error_message")?,
        attempts: row.get("attempts")?,
        output_path: row.get("output_path")?,
        library_id: row.get("library_id")?,
        position: row.get("position")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        finished_at: row.get("finished_at")?,
    })
}

/// Insere um job novo (a `position` já vem calculada).
pub fn insert(conn: &Connection, job: &Job) -> CoreResult<()> {
    conn.execute(
        &format!(
            "INSERT INTO jobs ({COLUMNS}) VALUES \
             (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32)"
        ),
        params![
            job.id,
            job.kind,
            job.provider,
            job.source_url,
            job.source_id,
            job.title,
            job.artist,
            job.thumbnail,
            job.duration_s,
            job.profile_id,
            serde_json::to_string(&job.options)?,
            job.metadata_override
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?,
            job.metadata_result
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?,
            job.confidence,
            serde_json::to_string(&job.warnings)?,
            job.playlist_ctx
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?,
            job.sync_id,
            job.status.as_str(),
            job.stage.as_str(),
            job.progress,
            job.overall_progress,
            job.speed_bps,
            job.eta_s,
            job.error_kind,
            job.error_message,
            job.attempts,
            job.output_path,
            job.library_id,
            job.position,
            job.created_at,
            job.updated_at,
            job.finished_at,
        ],
    )?;
    Ok(())
}

/// Grava os campos mutáveis do job (estado, progresso, erro, resultado, posição).
pub fn save(conn: &Connection, job: &Job) -> CoreResult<()> {
    conn.execute(
        "UPDATE jobs SET title=?2, artist=?3, duration_s=?4, status=?5, stage=?6, progress=?7, \
         overall_progress=?8, speed_bps=?9, eta_s=?10, error_kind=?11, error_message=?12, \
         attempts=?13, output_path=?14, library_id=?15, position=?16, updated_at=?17, \
         finished_at=?18, warnings_json=?19, source_url=?20, source_id=?21, \
         metadata_result_json=?22, confidence=?23 WHERE id=?1",
        params![
            job.id,
            job.title,
            job.artist,
            job.duration_s,
            job.status.as_str(),
            job.stage.as_str(),
            job.progress,
            job.overall_progress,
            job.speed_bps,
            job.eta_s,
            job.error_kind,
            job.error_message,
            job.attempts,
            job.output_path,
            job.library_id,
            job.position,
            job.updated_at,
            job.finished_at,
            serde_json::to_string(&job.warnings)?,
            job.source_url,
            job.source_id,
            job.metadata_result
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?,
            job.confidence,
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> CoreResult<Option<Job>> {
    let mut stmt = conn.prepare_cached(&format!("SELECT {COLUMNS} FROM jobs WHERE id = ?1"))?;
    let mut rows = stmt.query([id])?;
    rows.next()?.map(from_row).transpose()
}

/// Todos os jobs, na ordem da fila (`position`, depois criação).
pub fn list(conn: &Connection) -> CoreResult<Vec<Job>> {
    let mut stmt = conn.prepare_cached(&format!(
        "SELECT {COLUMNS} FROM jobs ORDER BY position, created_at, id"
    ))?;
    let mut rows = stmt.query([])?;
    let mut jobs = Vec::new();
    while let Some(row) = rows.next()? {
        jobs.push(from_row(row)?);
    }
    Ok(jobs)
}

/// O primeiro job `queued` por `position` que não esteja em `skip` (jobs esperando retry).
pub fn next_queued(conn: &Connection, skip: &HashSet<String>) -> CoreResult<Option<Job>> {
    let mut stmt = conn.prepare_cached(&format!(
        "SELECT {COLUMNS} FROM jobs WHERE status = 'queued' ORDER BY position, created_at, id"
    ))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let job = from_row(row)?;
        if !skip.contains(&job.id) {
            return Ok(Some(job));
        }
    }
    Ok(None)
}

/// Reserva o job para rodar: só tem efeito se ele ainda estiver `queued` (corrida com cancelamento).
pub fn claim(conn: &Connection, id: &str) -> CoreResult<Option<Job>> {
    let changed = conn.execute(
        "UPDATE jobs SET status='running', stage='downloading', attempts=attempts+1, progress=0, \
         overall_progress=0, speed_bps=NULL, eta_s=NULL, error_kind=NULL, error_message=NULL, \
         finished_at=NULL, updated_at=?2 WHERE id=?1 AND status='queued'",
        params![id, now()],
    )?;
    if changed == 0 {
        return Ok(None);
    }
    get(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> CoreResult<bool> {
    Ok(conn.execute("DELETE FROM jobs WHERE id = ?1", [id])? > 0)
}

/// Remove `done`/`failed`/`cancelled` e devolve os ids removidos.
pub fn delete_finished(conn: &Connection) -> CoreResult<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT id FROM jobs WHERE status IN ('done','failed','cancelled')")?;
    let ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    drop(stmt);
    conn.execute(
        "DELETE FROM jobs WHERE status IN ('done','failed','cancelled')",
        [],
    )?;
    Ok(ids)
}

/// Jobs que ainda ocupam a fila (`queued` + `running`).
pub fn count_active(conn: &Connection) -> CoreResult<u32> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM jobs WHERE status IN ('queued','running')",
        [],
        |row| row.get(0),
    )?)
}

pub fn count_status(conn: &Connection, status: JobStatus) -> CoreResult<u32> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM jobs WHERE status = ?1",
        [status.as_str()],
        |row| row.get(0),
    )?)
}

/// Próxima posição no fim da fila.
pub fn next_position(conn: &Connection) -> CoreResult<i64> {
    let max: Option<i64> = conn.query_row("SELECT MAX(position) FROM jobs", [], |r| r.get(0))?;
    Ok(max.map_or(1, |m| m + 1))
}

/// Posição "na frente de todos" (`min - 1`).
pub fn front_position(conn: &Connection) -> CoreResult<i64> {
    let min: Option<i64> = conn.query_row("SELECT MIN(position) FROM jobs", [], |r| r.get(0))?;
    Ok(min.map_or(1, |m| m - 1))
}

/// Reordena um job `queued`: renumera as posições dos jobs `queued` na nova ordem, reaproveitando
/// o conjunto de posições que eles já ocupavam. Devolve os jobs cuja posição mudou.
pub fn move_job(conn: &mut Connection, id: &str, target: &MoveTarget) -> CoreResult<Vec<Job>> {
    let tx = conn.transaction()?;
    let queued: Vec<Job> = list(&tx)?
        .into_iter()
        .filter(|j| j.status == JobStatus::Queued)
        .collect();
    if !queued.iter().any(|j| j.id == id) {
        return Err(CoreError::invalid(format!(
            "só é possível reordenar um job na fila: {id}"
        )));
    }
    let mut slots: Vec<i64> = queued.iter().map(|j| j.position).collect();
    slots.sort_unstable();
    slots.dedup();
    let mut ids: Vec<String> = queued.iter().map(|j| j.id.clone()).collect();
    ids.retain(|other| other != id);
    let index = match target {
        MoveTarget::Front => 0,
        MoveTarget::Back => ids.len(),
        MoveTarget::Before(other) | MoveTarget::After(other) => {
            let at = ids.iter().position(|x| x == other).ok_or_else(|| {
                CoreError::invalid(format!("job de referência não está na fila: {other}"))
            })?;
            if matches!(target, MoveTarget::Before(_)) {
                at
            } else {
                at + 1
            }
        }
    };
    ids.insert(index, id.to_string());
    // Se houver posições repetidas, abre espaço contínuo a partir da menor.
    let base = slots.first().copied().unwrap_or(1);
    let mut changed = Vec::new();
    for (offset, job_id) in ids.iter().enumerate() {
        let position = base + offset as i64;
        let current = queued.iter().find(|j| &j.id == job_id).map(|j| j.position);
        if current != Some(position) {
            tx.execute(
                "UPDATE jobs SET position = ?2 WHERE id = ?1",
                params![job_id, position],
            )?;
            if let Some(job) = get(&tx, job_id)? {
                changed.push(job);
            }
        }
    }
    // Posições além do bloco de `queued` (running/done com position maior) podem colidir; a ordem
    // de leitura desempata por `created_at`, então a colisão é inofensiva.
    tx.commit()?;
    Ok(changed)
}

/// Restauração na inicialização: `running ⇒ queued` e fim das esperas de retry (mesmas tentativas).
pub fn restore(conn: &Connection) -> CoreResult<u32> {
    let restored = conn.execute(
        "UPDATE jobs SET status='queued', stage='waiting', speed_bps=NULL, eta_s=NULL, \
         progress=0, overall_progress=0, updated_at=?1 WHERE status='running'",
        [now()],
    )?;
    conn.execute(
        "UPDATE jobs SET stage='waiting' WHERE status='queued' AND stage='waiting_retry'",
        [],
    )?;
    Ok(restored as u32)
}

/// Duplicatas por `(provider, source_id, profile_id)` em `jobs` (menos falhos/cancelados) e `library`.
pub fn find_duplicates(
    conn: &Connection,
    provider: &str,
    source_ids: &[String],
    profile_id: &str,
) -> CoreResult<Vec<DuplicateHit>> {
    let mut hits = Vec::new();
    let mut jobs = conn.prepare_cached(
        "SELECT id FROM jobs WHERE provider = ?1 AND source_id = ?2 AND profile_id = ?3 \
         AND status NOT IN ('failed','cancelled') LIMIT 1",
    )?;
    let mut library = conn.prepare_cached(
        "SELECT 1 FROM library WHERE provider = ?1 AND source_id = ?2 AND profile_id = ?3 LIMIT 1",
    )?;
    for source_id in source_ids {
        let job_id: Option<String> = jobs
            .query_row(params![provider, source_id, profile_id], |r| r.get(0))
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        if job_id.is_some() {
            hits.push(DuplicateHit {
                source_id: source_id.clone(),
                found_in: "queue".to_string(),
                job_id,
            });
            continue;
        }
        let in_library = library
            .query_row(params![provider, source_id, profile_id], |_| Ok(()))
            .map(|()| true)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(false),
                other => Err(other),
            })?;
        if in_library {
            hits.push(DuplicateHit {
                source_id: source_id.clone(),
                found_in: "library".to_string(),
                job_id: None,
            });
        }
    }
    Ok(hits)
}

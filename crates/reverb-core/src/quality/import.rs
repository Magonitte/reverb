//! Imported tags keep priority; acoustic identities add duplicate warnings.
use crate::library::files::ImportFailure;
use crate::{CoreResult, Db};
use std::path::{Path, PathBuf};
pub async fn identify(
    db: &Db,
    roots: Vec<PathBuf>,
    fpcalc: &Path,
    key: &str,
) -> CoreResult<Vec<ImportFailure>> {
    let items=db.call(move|conn|{
        let mut stmt=conn.prepare("SELECT id FROM library WHERE origin='import' AND acoustid_id IS NULL AND missing=0")?;
        let ids=stmt.query_map([],|r|r.get::<_,i64>(0))?.collect::<Result<Vec<_>,_>>()?;
        let mut items=Vec::new();for id in ids {if let Some(item)=crate::library::get(conn,id)?{if roots.iter().any(|root|Path::new(&item.file_path).starts_with(root)){items.push(item);}}}Ok(items)
    }).await?;
    let mut warnings = Vec::new();
    for item in items {
        let candidates = match super::acoustid::lookup(
            fpcalc,
            Path::new(&item.file_path),
            key,
            "https://api.acoustid.org/v2/lookup",
        )
        .await
        {
            Ok(candidates) => candidates,
            Err(_) => {
                warnings.push(ImportFailure {
                    path: item.file_path,
                    message: "warnings.fingerprint".into(),
                });
                continue;
            }
        };
        let Some(best) = candidates.first() else {
            continue;
        };
        let id = item.id;
        let sound = best.candidate.provider_id.clone();
        let recording = best.candidate.mb_recording_id.clone();
        let json = serde_json::to_string(&candidates)?;
        let duplicate=db.call(move|conn|{let tx=conn.transaction()?;let duplicates=crate::library::find_by_fingerprint(&tx,Some(&sound),recording.as_deref())?;let duplicate=duplicates.iter().any(|i|i.id!=id);tx.execute("UPDATE library SET acoustid_id=?,mb_recording_id=?,needs_review=CASE WHEN ? THEN 1 ELSE needs_review END,review_candidates_json=CASE WHEN ? THEN ? ELSE review_candidates_json END,updated_at=unixepoch() WHERE id=?",rusqlite::params![sound,recording,duplicate,duplicate,json,id])?;tx.commit()?;Ok(duplicate)}).await?;
        if duplicate {
            warnings.push(ImportFailure {
                path: item.file_path,
                message: "errors.duplicate".into(),
            });
        }
    }
    Ok(warnings)
}

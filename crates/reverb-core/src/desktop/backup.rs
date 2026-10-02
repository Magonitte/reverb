//! Online SQLite backups and redacted diagnostics archives. Audio is never copied.
use super::diagnostics::DiagnosticReport;
use crate::{CoreError, CoreResult, Db, Settings};
use rusqlite::{Connection, OpenFlags};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

const SECRET_KEYS: [&str; 5] = [
    "acoustidKey",
    "spotifyClientId",
    "spotifyClientSecret",
    "discogsToken",
    "jamendoClientId",
];
const MAX_DB: u64 = 512 * 1024 * 1024;

fn error(error: impl std::fmt::Display) -> CoreError {
    CoreError::coded("backup", error.to_string())
}
fn remove_secrets(conn: &Connection, settings: &serde_json::Value) -> CoreResult<()> {
    // Use the same sanitized snapshot in both exported entries, and vacuum deleted values.
    conn.execute("DELETE FROM settings", [])?;
    for (key, value) in settings
        .as_object()
        .ok_or_else(|| CoreError::invalid("Invalid settings"))?
    {
        conn.execute(
            "INSERT INTO settings(key,value) VALUES (?,?)",
            (key, serde_json::to_string(value)?),
        )?;
    }
    conn.execute_batch("PRAGMA secure_delete=ON; VACUUM;")?;
    Ok(())
}
fn safe_settings(settings: &Settings) -> CoreResult<serde_json::Value> {
    let mut view = serde_json::to_value(settings.view())?;
    if let Some(view) = view.as_object_mut() {
        view.remove("secretsStatus");
        view.insert("cookiesSource".into(), serde_json::json!("none"));
        view.insert("cookiesFile".into(), serde_json::json!(""));
    }
    Ok(view)
}
fn write_archive(path: &Path, entries: Vec<(String, Vec<u8>)>) -> CoreResult<()> {
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        return Err(CoreError::invalid("Choose a ZIP destination"));
    }
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| CoreError::invalid("Choose an absolute backup destination"))?;
    if !path.is_absolute() {
        return Err(CoreError::invalid("Choose an absolute backup destination"));
    }
    std::fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut zip = zip::ZipWriter::new(temporary.as_file_mut());
        for (name, bytes) in entries {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .map_err(error)?;
            zip.write_all(&bytes)?;
        }
        zip.finish().map_err(error)?;
    }
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(error)?;
    Ok(())
}

pub async fn data_export(db: &Db, settings: &Settings, path: &Path) -> CoreResult<()> {
    let scratch = tempfile::tempdir()?;
    let snapshot = scratch.path().join("reverb.db");
    let target = snapshot.clone();
    let sanitized = safe_settings(settings)?;
    let stored_settings = sanitized.clone();
    db.call(move |conn| {
        let mut output = Connection::open(&target)?;
        rusqlite::backup::Backup::new(conn, &mut output)?.run_to_completion(
            100,
            Duration::from_millis(10),
            None,
        )?;
        remove_secrets(&output, &stored_settings)?;
        Ok(())
    })
    .await?;
    let entries = vec![
        ("reverb.db".into(), std::fs::read(snapshot)?),
        (
            "settings.json".into(),
            serde_json::to_vec_pretty(&sanitized)?,
        ),
    ];
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || write_archive(&path, entries))
        .await
        .map_err(error)?
}

/// Validated temporary snapshot, held until the live connection can be restored.
pub struct ValidatedBackup(tempfile::TempDir);

fn schema(conn: &Connection) -> CoreResult<Vec<(String, String, Option<String>)>> {
    let mut query = conn.prepare(
        "SELECT type,name,sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name",
    )?;
    let rows = query
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

pub async fn prepare_import(path: &Path) -> CoreResult<ValidatedBackup> {
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || -> CoreResult<ValidatedBackup> {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(path)?).map_err(error)?;
        if zip.len() != 2 {
            return Err(CoreError::invalid(
                "Backup must contain exactly reverb.db and settings.json",
            ));
        }
        let mut config = String::new();
        zip.by_name("settings.json")
            .map_err(error)?
            .take(1024 * 1024 + 1)
            .read_to_string(&mut config)?;
        if config.len() > 1024 * 1024 {
            return Err(CoreError::invalid("Backup settings are too large"));
        }
        let json: serde_json::Value = serde_json::from_str(&config)?;
        let values = json
            .as_object()
            .ok_or_else(|| CoreError::invalid("Invalid backup settings"))?;
        if SECRET_KEYS.iter().any(|key| values.contains_key(*key))
            || values.get("cookiesSource") != Some(&serde_json::json!("none"))
            || values.get("cookiesFile") != Some(&serde_json::json!(""))
        {
            return Err(CoreError::invalid("Backup contains secrets or cookies"));
        }
        let patch: crate::SettingsPatch = serde_json::from_value(json.clone())?;
        let mut settings = Settings::default();
        settings.apply(patch);
        crate::settings::validate(&settings)?;
        if safe_settings(&settings)? != json {
            return Err(CoreError::invalid("Backup settings are incomplete"));
        }
        let scratch = tempfile::tempdir()?;
        let source = scratch.path().join("reverb.db");
        let mut entry = zip.by_name("reverb.db").map_err(error)?;
        if entry.size() > MAX_DB {
            return Err(CoreError::invalid("Backup database is too large"));
        }
        let mut file = std::fs::File::create(&source)?;
        let written = std::io::copy(&mut entry.by_ref().take(MAX_DB + 1), &mut file)?;
        if written > MAX_DB {
            return Err(CoreError::invalid("Backup database is too large"));
        }
        file.sync_all()?;
        let conn = Connection::open_with_flags(&source, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        conn.execute_batch("PRAGMA trusted_schema=OFF;")?;
        let expected = Connection::open_in_memory()?;
        expected.execute_batch(include_str!("../../migrations/0001_init.sql"))?;
        if schema(&conn)? != schema(&expected)? {
            return Err(CoreError::invalid("Backup schema is incompatible"));
        }
        let integrity: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        let foreign_error = conn.prepare("PRAGMA foreign_key_check")?.exists([])?;
        if integrity != "ok" || version != 1 || foreign_error {
            return Err(CoreError::invalid(
                "Corrupt or incompatible backup database",
            ));
        }
        let mut query = conn.prepare("SELECT key,value FROM settings")?;
        let rows: Vec<(String, String)> = query
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let mut stored = serde_json::Map::new();
        for (key, value) in rows {
            stored.insert(key, serde_json::from_str(&value)?);
        }
        if serde_json::Value::Object(stored) != json {
            return Err(CoreError::invalid(
                "Backup settings do not match the database",
            ));
        }
        Ok(ValidatedBackup(scratch))
    })
    .await
    .map_err(error)?
}

pub async fn data_import(
    db: &Db,
    current: &Settings,
    data_dir: &Path,
    path: &Path,
) -> CoreResult<PathBuf> {
    let source = prepare_import(path).await?;
    restore(db, current, data_dir, source).await
}

pub async fn restore(
    db: &Db,
    current: &Settings,
    data_dir: &Path,
    source: ValidatedBackup,
) -> CoreResult<PathBuf> {
    let backup = data_dir
        .join("backups")
        .join(format!("before-restore-{}.zip", uuid::Uuid::new_v4()));
    data_export(db, current, &backup).await?;
    let source_path = source.0.path().join("reverb.db");
    db.call(move |conn| {
        let source = Connection::open_with_flags(source_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        rusqlite::backup::Backup::new(&source, conn)?.run_to_completion(
            100,
            Duration::from_millis(10),
            None,
        )?;
        Ok(())
    })
    .await?;
    Ok(backup)
}

pub async fn logs_export(log_dir: &Path, report: &DiagnosticReport, path: &Path) -> CoreResult<()> {
    let report = crate::logging::redact(&serde_json::to_string_pretty(report)?);
    let dir = log_dir.to_owned();
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || {
        let mut entries = vec![("diagnostics.json".into(), report.into_bytes())];
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let file_type = entry.file_type()?;
                if !file_type.is_file()
                    || !entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(crate::logging::LOG_FILE_PREFIX)
                {
                    continue;
                }
                if entry.metadata()?.len() > 32 * 1024 * 1024 {
                    continue;
                }
                let bytes = std::fs::read(entry.path())?;
                let text = crate::logging::redact(&String::from_utf8_lossy(&bytes));
                entries.push((
                    format!("logs/{}", entry.file_name().to_string_lossy()),
                    text.into_bytes(),
                ));
            }
        }
        write_archive(&path, entries)
    })
    .await
    .map_err(error)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{events::MemorySink, SettingsService};
    #[tokio::test]
    async fn mismatched_settings_or_modified_schema_are_rejected_before_backup() {
        let dir = tempfile::tempdir().unwrap();
        let source = Db::open_in_memory().unwrap();
        let path = dir.path().join("valid.zip");
        data_export(&source, &Settings::default(), &path)
            .await
            .unwrap();
        let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut database = Vec::new();
        zip.by_name("reverb.db")
            .unwrap()
            .read_to_end(&mut database)
            .unwrap();
        let config = serde_json::to_vec(&safe_settings(&Settings::default()).unwrap()).unwrap();
        let target = Db::open_in_memory().unwrap();
        target.kv_set("keep", "original").await.unwrap();
        let mut changed: serde_json::Value = serde_json::from_slice(&config).unwrap();
        changed["theme"] = serde_json::json!("light");
        let bad_config = dir.path().join("mismatch.zip");
        write_archive(
            &bad_config,
            vec![
                ("reverb.db".into(), database.clone()),
                (
                    "settings.json".into(),
                    serde_json::to_vec(&changed).unwrap(),
                ),
            ],
        )
        .unwrap();
        assert!(
            data_import(&target, &Settings::default(), dir.path(), &bad_config)
                .await
                .is_err()
        );
        let db_path = dir.path().join("tampered.db");
        std::fs::write(&db_path, database).unwrap();
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch("DROP TRIGGER library_fts_ai;").unwrap();
        drop(conn);
        let bad_schema = dir.path().join("schema.zip");
        write_archive(
            &bad_schema,
            vec![
                ("reverb.db".into(), std::fs::read(db_path).unwrap()),
                ("settings.json".into(), config),
            ],
        )
        .unwrap();
        assert!(
            data_import(&target, &Settings::default(), dir.path(), &bad_schema)
                .await
                .is_err()
        );
        assert_eq!(
            target.kv_get("keep").await.unwrap().as_deref(),
            Some("original")
        );
        assert!(!dir.path().join("backups").exists());
    }
    #[tokio::test]
    async fn t7_online_backup_restores_library_settings_without_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("source.db")).unwrap();
        let settings = SettingsService::new(db.clone(), std::sync::Arc::new(MemorySink::new()))
            .await
            .unwrap();
        settings.update(serde_json::from_value(serde_json::json!({"theme":"light","acoustidKey":"secret-export-123","cookiesFile":"never-export-cookie-file"})).unwrap()).await.unwrap();
        db.call(|conn|{conn.execute("INSERT INTO library(file_path,title,added_at,updated_at) VALUES('/test/audio.opus','Backup track',1,1)",[])?;Ok(())}).await.unwrap();
        let archive = dir.path().join("backup.zip");
        data_export(&db, &settings.get(), &archive).await.unwrap();
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&archive).unwrap()).unwrap();
        for n in 0..zip.len() {
            let mut bytes = vec![];
            zip.by_index(n).unwrap().read_to_end(&mut bytes).unwrap();
            let text = String::from_utf8_lossy(&bytes);
            assert!(!text.contains("secret-export-123"));
            assert!(!text.contains("never-export-cookie-file"));
        }
        let target = Db::open(&dir.path().join("target/reverb.db")).unwrap();
        let backup = data_import(
            &target,
            &Settings::default(),
            &dir.path().join("target"),
            &archive,
        )
        .await
        .unwrap();
        assert!(backup.is_file());
        let restored = SettingsService::new(target.clone(), std::sync::Arc::new(MemorySink::new()))
            .await
            .unwrap();
        assert_eq!(restored.get().theme, crate::settings::Theme::Light);
        assert!(restored.get().acoustid_key.is_empty());
        assert_eq!(
            target
                .call(|conn| Ok(conn
                    .query_row("SELECT title FROM library", [], |row| row
                        .get::<_, String>(0))?))
                .await
                .unwrap(),
            "Backup track"
        );
    }
    #[tokio::test]
    async fn t6_logs_archive_redacts_configured_secrets_and_cookie_headers() {
        let dir = tempfile::tempdir().unwrap();
        crate::logging::register_secret("configured-API-value-123");
        std::fs::write(dir.path().join(format!("{}2026-10-02",crate::logging::LOG_FILE_PREFIX)),"configured-API-value-123\nCookie: session=literal-cookie-value\nhttps://a.test/?token=signed-url-token").unwrap();
        std::fs::write(dir.path().join("cookies.txt"), "Must never be included").unwrap();
        let output = dir.path().join("export.zip");
        logs_export(
            dir.path(),
            &DiagnosticReport {
                created_at: 1,
                items: vec![],
            },
            &output,
        )
        .await
        .unwrap();
        let mut zip = zip::ZipArchive::new(std::fs::File::open(output).unwrap()).unwrap();
        assert_eq!(zip.len(), 2);
        for n in 0..zip.len() {
            let mut text = String::new();
            zip.by_index(n).unwrap().read_to_string(&mut text).unwrap();
            for secret in [
                "configured-API-value-123",
                "literal-cookie-value",
                "signed-url-token",
                "Must never be included",
            ] {
                assert!(!text.contains(secret), "{secret}");
            }
        }
    }
    #[tokio::test]
    async fn corrupt_archive_does_not_modify_live_database() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open_in_memory().unwrap();
        db.kv_set("keep", "original").await.unwrap();
        let path = dir.path().join("invalid.zip");
        write_archive(
            &path,
            vec![
                ("reverb.db".into(), b"invalid".to_vec()),
                ("settings.json".into(), b"{}".to_vec()),
            ],
        )
        .unwrap();
        assert!(data_import(&db, &Settings::default(), dir.path(), &path)
            .await
            .is_err());
        assert_eq!(
            db.kv_get("keep").await.unwrap().as_deref(),
            Some("original")
        );
        assert!(!dir.path().join("backups").exists());
    }
}

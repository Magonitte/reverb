use super::*;
use tempfile::tempdir;

const TABLES: &[&str] = &[
    "settings",
    "kv",
    "syncs",
    "library",
    "library_fts",
    "jobs",
    "sync_items",
    "followed_artists",
    "followed_releases",
    "metadata_cache",
];

fn user_version(db: &Db) -> i64 {
    db.call_blocking(|c| Ok(c.query_row("PRAGMA user_version", [], |r| r.get(0))?))
        .unwrap()
}

fn insert_track(db: &Db, path: &str, title: &str) -> i64 {
    let (path, title) = (path.to_string(), title.to_string());
    db.call_blocking(move |c| {
        c.execute(
            "INSERT INTO library (file_path, title, artist, album, added_at, updated_at) \
             VALUES (?1, ?2, 'Artista', 'Álbum', 0, 0)",
            [&path, &title],
        )?;
        Ok(c.last_insert_rowid())
    })
    .unwrap()
}

fn fts_matches(db: &Db, query: &str) -> Vec<String> {
    let query = query.to_string();
    db.call_blocking(move |c| {
        let mut stmt = c.prepare(
            "SELECT l.title FROM library_fts f JOIN library l ON l.id = f.rowid \
             WHERE library_fts MATCH ?1 ORDER BY l.id",
        )?;
        let rows = stmt
            .query_map([&query], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
    .unwrap()
}

#[test]
fn t1_migracao_em_banco_vazio_cria_tudo() {
    let db = Db::open_in_memory().unwrap();
    assert_eq!(user_version(&db), 1);
    let names: Vec<String> = db
        .call_blocking(|c| {
            let mut stmt = c.prepare("SELECT name FROM sqlite_master WHERE type IN ('table')")?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
        .unwrap();
    for table in TABLES {
        assert!(names.iter().any(|n| n == table), "falta a tabela {table}");
    }
}

#[test]
fn t2_migracoes_sao_idempotentes() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("reverb.db");
    drop(Db::open(&path).unwrap());
    let db = Db::open(&path).unwrap();
    assert_eq!(user_version(&db), 1);
    db.call_blocking(|c| {
        migrate(c)?;
        migrate(c)
    })
    .unwrap();
    assert_eq!(user_version(&db), 1);
}

#[test]
fn t3_fts5_busca_sem_diacriticos_e_acompanha_update_e_delete() {
    let db = Db::open_in_memory().unwrap();
    let id = insert_track(&db, "a.opus", "Ação Música");
    insert_track(&db, "b.opus", "Outra Coisa");
    insert_track(&db, "c.opus", "Mais Uma");

    assert_eq!(fts_matches(&db, "acao"), vec!["Ação Música"]);
    assert_eq!(fts_matches(&db, "musica"), vec!["Ação Música"]);

    db.call_blocking(move |c| {
        c.execute(
            "UPDATE library SET title = 'Canção Nova' WHERE id = ?1",
            [id],
        )?;
        Ok(())
    })
    .unwrap();
    assert!(fts_matches(&db, "acao").is_empty());
    assert_eq!(fts_matches(&db, "cancao"), vec!["Canção Nova"]);

    db.call_blocking(move |c| {
        c.execute("DELETE FROM library WHERE id = ?1", [id])?;
        Ok(())
    })
    .unwrap();
    assert!(fts_matches(&db, "cancao").is_empty());
    assert_eq!(fts_matches(&db, "coisa"), vec!["Outra Coisa"]);
}

#[test]
fn t4_foreign_keys_ativas_e_cascata_em_syncs() {
    let db = Db::open_in_memory().unwrap();
    let remaining = db
        .call_blocking(|c| {
            let fk: i64 = c.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?;
            assert_eq!(fk, 1);
            c.execute(
                "INSERT INTO syncs (id, url, title, profile_id, created_at) \
                 VALUES ('s1', 'u', 't', 'original', 0)",
                [],
            )?;
            c.execute(
                "INSERT INTO sync_items (sync_id, source_id, position, state, first_seen_at) \
                 VALUES ('s1', 'v1', 0, 'present', 0)",
                [],
            )?;
            c.execute("DELETE FROM syncs WHERE id = 's1'", [])?;
            Ok(c.query_row("SELECT COUNT(*) FROM sync_items", [], |r| {
                r.get::<_, i64>(0)
            })?)
        })
        .unwrap();
    assert_eq!(remaining, 0);
}

#[test]
fn wal_e_busy_timeout_configurados() {
    let dir = tempdir().unwrap();
    let db = Db::open(&dir.path().join("sub").join("reverb.db")).unwrap();
    let (mode, timeout) = db
        .call_blocking(|c| {
            Ok((
                c.query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))?,
                c.query_row("PRAGMA busy_timeout", [], |r| r.get::<_, i64>(0))?,
            ))
        })
        .unwrap();
    assert_eq!(mode, "wal");
    assert_eq!(timeout, 5000);
}

#[tokio::test]
async fn call_assincrono_executa_na_conexao() {
    let db = Db::open_in_memory().unwrap();
    let n = db
        .call(|c| Ok(c.query_row("SELECT 40 + 2", [], |r| r.get::<_, i64>(0))?))
        .await
        .unwrap();
    assert_eq!(n, 42);
}

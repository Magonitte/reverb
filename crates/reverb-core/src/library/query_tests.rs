use super::*;
use crate::Db;

#[test]
fn invalid_cover_sizes_fail_before_reading_the_file() {
    for size in [0, 1201, u32::MAX] {
        assert!(crate::library::cover_thumbnail_with_size(
            std::path::Path::new("missing.opus"),
            size
        )
        .is_err());
    }
}

fn seed(conn: &Connection, path: &str, title: &str, artist: &str, album: &str, age: i64) -> i64 {
    conn.execute("INSERT INTO library(file_path,title,artist,album,added_at,updated_at,provider,source_id,isrc,needs_review) VALUES(?,?,?,?,unixepoch()-?,unixepoch(),'youtube',?,'ISRC',1)", rusqlite::params![path,title,artist,album,age,path]).unwrap();
    conn.last_insert_rowid()
}

#[test]
fn accented_prefix_and_combined_filters() {
    Db::open_in_memory()
        .unwrap()
        .call_blocking(|conn| {
            seed(conn, "a.opus", "Música Never", "Rick", "Album", 0);
            seed(conn, "b.mp3", "Música Never", "Rick", "Album", 0);
            seed(conn, "c.opus", "Música Never", "Other", "Album", 0);
            seed(conn, "d.opus", "Música Never", "Rick", "Singles", 0);
            seed(conn, "e.opus", "Música Never", "Rick", "Album", 40 * 86400);
            let query = LibraryQuery {
                text: Some("musica nev".into()),
                artist: Some("Rick".into()),
                album: Some("Album".into()),
                format: Some("opus".into()),
                date_range: LibraryDateRange::Week,
                needs_review: Some(true),
                missing: Some(false),
                ..Default::default()
            };
            let result = list(conn, &query)?;
            assert_eq!(result.total, 1);
            assert_eq!(result.items[0].file_path, "a.opus");
            Ok(())
        })
        .unwrap();
}

#[test]
fn stable_pagination_and_all_sort_orders() {
    Db::open_in_memory()
        .unwrap()
        .call_blocking(|conn| {
            for n in 0..12 {
                seed(conn, &format!("{n}.opus"), "Same", "Same", "Same", 0);
            }
            for sort in [
                LibrarySort::AddedDesc,
                LibrarySort::Title,
                LibrarySort::Artist,
                LibrarySort::Album,
            ] {
                let full = list(
                    conn,
                    &LibraryQuery {
                        sort,
                        ..Default::default()
                    },
                )?;
                let a = list(
                    conn,
                    &LibraryQuery {
                        sort,
                        limit: 5,
                        ..Default::default()
                    },
                )?;
                let b = list(
                    conn,
                    &LibraryQuery {
                        sort,
                        limit: 7,
                        offset: 5,
                        ..Default::default()
                    },
                )?;
                assert_eq!(a.total, 12);
                assert_eq!([a.items, b.items].concat(), full.items);
            }
            Ok(())
        })
        .unwrap();
}

#[test]
fn user_text_cannot_be_fts_or_sql_syntax() {
    Db::open_in_memory()
        .unwrap()
        .call_blocking(|conn| {
            seed(conn, "a.flac", "Never", "Rick", "Album", 0);
            for text in [
                "\"",
                "*",
                "title:Never",
                "never OR missing",
                "'; DROP TABLE library; --",
            ] {
                assert_eq!(
                    list(
                        conn,
                        &LibraryQuery {
                            text: Some(text.into()),
                            ..Default::default()
                        }
                    )?
                    .total,
                    0
                );
            }
            assert_eq!(
                list(
                    conn,
                    &LibraryQuery {
                        text: Some("  ".into()),
                        ..Default::default()
                    }
                )?
                .total,
                1
            );
            assert!(list(
                conn,
                &LibraryQuery {
                    limit: 0,
                    ..Default::default()
                }
            )
            .is_err());
            assert!(list(
                conn,
                &LibraryQuery {
                    limit: 501,
                    ..Default::default()
                }
            )
            .is_err());
            assert!(list(
                conn,
                &LibraryQuery {
                    format: Some("%".into()),
                    ..Default::default()
                }
            )
            .is_err());
            Ok(())
        })
        .unwrap();
}

#[test]
fn facets_source_isrc_get_and_review_dismiss() {
    Db::open_in_memory()
        .unwrap()
        .call_blocking(|conn| {
            let id = seed(conn, "a.flac", "Song", "Rick", "Album", 0);
            seed(conn, "b.flac", "Other", "Jane", "Other", 0);
            seed(conn, "c.flac", "More", "Rick", "Album", 0);
            assert_eq!(artists(conn)?, vec!["Jane", "Rick"]);
            assert_eq!(albums(conn, Some("Rick"))?, vec!["Album"]);
            assert!(get(conn, 999)?.is_none());
            dismiss(conn, id)?;
            assert!(!get(conn, id)?.unwrap().needs_review);
            assert!(dismiss(conn, 999).is_err());
            Ok(())
        })
        .unwrap();
}

#[test]
fn removing_records_preserves_files_and_cleans_fts_and_job_links() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("track.opus");
    std::fs::write(&path, b"audio").unwrap();
    Db::open_in_memory().unwrap().call_blocking(|conn| {
        let id=seed(conn,&path.to_string_lossy(),"Never","Rick","Album",0);
        conn.execute("INSERT INTO jobs(id,kind,source_url,profile_id,status,stage,position,created_at,updated_at,library_id) VALUES('job','single','url','original','done','done',0,0,0,?)",[id])?;
        assert_eq!(delete(conn,&[id,id,999],false)?,1);
        assert!(path.exists());
        assert_eq!(list(conn,&LibraryQuery{text:Some("nev".into()),..Default::default()})?.total,0);
        let link:Option<i64>=conn.query_row("SELECT library_id FROM jobs",[],|r|r.get(0))?;
        assert_eq!(link,None);
        seed(conn,"second.opus","Never","Rick","Album",0);
        assert_eq!(clear(conn)?,1);
        assert_eq!(list(conn,&LibraryQuery::default())?.total,0);
        assert!(path.exists());
        Ok(())
    }).unwrap();
}

#[test]
fn trash_removes_audio_and_sibling_lyrics_without_deleting_folder_cover() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("track.opus");
    let lrc = path.with_extension("lrc");
    let cover = tmp.path().join("cover.jpg");
    for p in [&path, &lrc, &cover] {
        std::fs::write(p, b"fixture").unwrap();
    }
    Db::open_in_memory()
        .unwrap()
        .call_blocking(|conn| {
            let id = seed(conn, &path.to_string_lossy(), "Track", "Rick", "Album", 0);
            assert_eq!(delete(conn, &[id], true)?, 1);
            assert!(!path.exists());
            assert!(!lrc.exists());
            assert!(cover.exists());
            assert!(get(conn, id)?.is_none());
            Ok(())
        })
        .unwrap();
}

#[test]
fn date_ranges_and_missing_filter() {
    Db::open_in_memory()
        .unwrap()
        .call_blocking(|conn| {
            let id = seed(conn, "today.opus", "Today", "Rick", "Album", 0);
            seed(conn, "week.opus", "Week", "Rick", "Album", 3 * 86400);
            seed(conn, "month.opus", "Month", "Rick", "Album", 15 * 86400);
            seed(conn, "old.opus", "Old", "Rick", "Album", 60 * 86400);
            for (range, total) in [
                (LibraryDateRange::Today, 1),
                (LibraryDateRange::Week, 2),
                (LibraryDateRange::Month, 3),
                (LibraryDateRange::All, 4),
            ] {
                assert_eq!(
                    list(
                        conn,
                        &LibraryQuery {
                            date_range: range,
                            ..Default::default()
                        }
                    )?
                    .total,
                    total
                );
            }
            conn.execute("UPDATE library SET missing=1 WHERE id=?", [id])?;
            assert_eq!(
                list(
                    conn,
                    &LibraryQuery {
                        missing: Some(true),
                        ..Default::default()
                    }
                )?
                .items[0]
                    .id,
                id
            );
            Ok(())
        })
        .unwrap();
}

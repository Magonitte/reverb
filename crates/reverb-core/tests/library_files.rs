//! F10: real containers, isolated folders, and filesystem notifications.
mod common;
use reverb_core::events::MemorySink;
use reverb_core::library::{self, files, LibraryQuery};
use reverb_core::tagging::{self, TrackTags};
use reverb_core::{Db, Settings, SettingsPatch, SettingsService};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

async fn audio(root: &Path, name: &str) -> PathBuf {
    std::fs::create_dir_all(root).unwrap();
    let path = root.join(name);
    let codec = match path.extension().unwrap().to_str().unwrap() {
        "mp3" => "libmp3lame",
        "m4a" => "aac",
        "opus" => "libopus",
        "ogg" => "libvorbis",
        "flac" => "flac",
        "wav" => "pcm_s16le",
        _ => unreachable!(),
    };
    let mut args = [
        "-hide_banner",
        "-nostdin",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:duration=0.5",
        "-c:a",
        codec,
    ]
    .map(String::from)
    .to_vec();
    args.push(path.to_string_lossy().into_owned());
    let result =
        reverb_core::tools::run_capture(common::ffmpeg(), args, |_| {}, Duration::from_secs(30))
            .await
            .unwrap();
    assert!(result.success, "{}", result.stderr);
    let tags = TrackTags {
        title: "Canção Música".into(),
        artist: Some("Artista".into()),
        album: Some("Álbum".into()),
        year: Some(2026),
        track_no: Some(2),
        ..Default::default()
    };
    tagging::write_tags(&path, &tags).unwrap();
    path
}
async fn page(db: &Db) -> library::LibraryPage {
    db.call(|conn| library::list(conn, &LibraryQuery::default()))
        .await
        .unwrap()
}
async fn import(db: &Db, paths: Vec<PathBuf>) -> files::ImportReport {
    files::import(
        db,
        paths,
        &common::ffprobe(),
        Arc::new(MemorySink::new()),
        "import",
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn six_formats_nested_idempotent_and_progress() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    for (index, ext) in ["mp3", "m4a", "opus", "ogg", "flac", "wav"]
        .iter()
        .enumerate()
    {
        audio(
            &dir.path().join(format!("sub/{index}")),
            &format!("track.{ext}"),
        )
        .await;
    }
    let sink = Arc::new(MemorySink::new());
    let result = files::import(
        &db,
        vec![dir.path().into()],
        &common::ffprobe(),
        sink.clone(),
        "import",
    )
    .await
    .unwrap();
    assert!(result.failures.is_empty(), "{:?}", result.failures);
    assert_eq!(result.imported, 6);
    let items = page(&db).await;
    assert_eq!(items.total, 6);
    for item in items.items {
        assert_eq!(item.title, "Canção Música");
        assert_eq!(item.artist.as_deref(), Some("Artista"));
        assert_eq!(item.album.as_deref(), Some("Álbum"));
        assert_eq!(item.year, Some(2026));
        assert_eq!(item.track_no, Some(2));
        assert_eq!(item.origin, "import");
        assert!(item.duration_s.unwrap() > 0.4);
    }
    assert_eq!(
        sink.named("library://import-progress").last().unwrap()["processed"],
        6
    );
    let second = import(&db, vec![dir.path().into(), dir.path().join("sub")]).await;
    assert_eq!(second.imported, 0);
    assert_eq!(second.skipped, 6);
    assert_eq!(page(&db).await.total, 6);
}

#[tokio::test]
async fn rescan_marks_removed_and_imports_new() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    let old = audio(dir.path(), "old.opus").await;
    import(&db, vec![dir.path().into()]).await;
    std::fs::remove_file(old).unwrap();
    audio(dir.path(), "new.flac").await;
    let result = files::rescan(
        &db,
        dir.path().into(),
        &common::ffprobe(),
        Arc::new(MemorySink::new()),
    )
    .await
    .unwrap();
    assert_eq!(result.imported, 1);
    let items = page(&db).await.items;
    assert_eq!(items.len(), 2);
    assert!(
        items
            .iter()
            .find(|i| i.file_path.ends_with("old.opus"))
            .unwrap()
            .missing
    );
    assert_eq!(
        items
            .iter()
            .find(|i| i.file_path.ends_with("new.flac"))
            .unwrap()
            .origin,
        "scan"
    );
}

#[tokio::test]
async fn external_edit_does_not_import_and_review_moves_audio_lyrics_updates_fts() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    let path = audio(dir.path(), "old.opus").await;
    let mut tags = tagging::read_tags(&path).unwrap();
    tags.title = "Edited external".into();
    let external = path.clone();
    let new_tags = tags.clone();
    db.call(move |conn| files::write(conn, &external, &new_tags, true, &Settings::default()))
        .await
        .unwrap();
    assert_eq!(page(&db).await.total, 0);
    import(&db, vec![path.clone()]).await;
    let id = page(&db).await.items[0].id;
    db.call(move |conn| {
        conn.execute("UPDATE library SET needs_review=1 WHERE id=?", [id])?;
        Ok(())
    })
    .await
    .unwrap();
    std::fs::write(path.with_extension("lrc"), "[00:01]letra").unwrap();
    tags.title = "Título revisado".into();
    tags.album = Some("Novo álbum".into());
    let expected = tags.clone();
    let settings = Settings {
        output_dir: dir.path().join("library").to_string_lossy().into_owned(),
        file_template: "{artist}/{album}/{title}".into(),
        auto_organize: true,
        ..Default::default()
    };
    let item = db
        .call(move |conn| files::apply_review(conn, id, &tags, &settings, "user"))
        .await
        .unwrap();
    assert!(!item.needs_review);
    assert_eq!(item.metadata_source.as_deref(), Some("user"));
    assert!(!path.exists());
    assert_eq!(
        tagging::read_tags(Path::new(&item.file_path)).unwrap(),
        expected
    );
    assert!(Path::new(&item.file_path).with_extension("lrc").exists());
    assert_eq!(
        db.call(|conn| library::list(
            conn,
            &LibraryQuery {
                text: Some("titulo revis".into()),
                ..Default::default()
            }
        ))
        .await
        .unwrap()
        .total,
        1
    );
    // Already organized edits retain their filename rather than adding a collision suffix.
    let filename = item.file_path.clone();
    let source = filename.clone();
    let settings = Settings {
        output_dir: dir.path().join("library").to_string_lossy().into_owned(),
        file_template: "{artist}/{album}/{title}".into(),
        auto_organize: true,
        ..Default::default()
    };
    assert_eq!(
        db.call(move |conn| files::write(conn, Path::new(&source), &expected, true, &settings))
            .await
            .unwrap()
            .to_string_lossy(),
        filename
    );
}

#[tokio::test]
async fn watcher_imports_within_five_seconds_and_defers_pipeline_paths() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    let sink = Arc::new(MemorySink::new());
    let settings = Arc::new(
        SettingsService::new(db.clone(), sink.clone())
            .await
            .unwrap(),
    );
    settings
        .update(SettingsPatch {
            output_dir: Some(dir.path().to_string_lossy().into_owned()),
            watch_library: Some(true),
            ..Default::default()
        })
        .await
        .unwrap();
    let cancel = CancellationToken::new();
    let task = tokio::spawn(library::watch::run(
        db.clone(),
        settings,
        sink,
        Arc::new(|| Ok(common::ffprobe())),
        cancel.clone(),
    ));
    tokio::time::sleep(Duration::from_millis(350)).await;
    let source = audio(outside.path(), "source.opus").await;
    let started = Instant::now();
    std::fs::copy(&source, dir.path().join("new.opus")).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while page(&db).await.total != 1 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    assert!(started.elapsed() <= Duration::from_secs(5));
    let target = dir.path().join("pipeline.opus");
    let mut moving = reverb_core::organize::MovingPaths::default();
    moving.reserve(&target);
    std::fs::copy(source, &target).unwrap();
    tokio::time::sleep(Duration::from_millis(3300)).await;
    assert_eq!(page(&db).await.total, 1);
    let path = files::canonical(&target)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    db.call(move |conn|{conn.execute("INSERT INTO library(file_path,title,origin,added_at,updated_at) VALUES(?,'Pipeline','download',unixepoch(),unixepoch())",[path])?;Ok(())}).await.unwrap();
    drop(moving);
    tokio::time::sleep(Duration::from_millis(2200)).await;
    assert_eq!(page(&db).await.total, 2);
    std::fs::remove_file(&target).unwrap();
    tokio::time::timeout(Duration::from_secs(4), async {
        while !page(&db)
            .await
            .items
            .iter()
            .any(|i| i.title == "Pipeline" && i.missing)
        {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    std::fs::copy(dir.path().join("new.opus"), &target).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while page(&db).await.items.iter().any(|i| i.missing) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(page(&db).await.total, 2);
    cancel.cancel();
    task.await.unwrap();
}

#[tokio::test]
async fn failed_database_edit_restores_audio_tags_and_lyrics() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_in_memory().unwrap();
    let path = audio(dir.path(), "original.opus").await;
    let original = tagging::read_tags(&path).unwrap();
    std::fs::write(path.with_extension("lrc"), "original lyrics").unwrap();
    import(&db, vec![path.clone()]).await;
    db.call(|conn|{conn.execute_batch("CREATE TRIGGER reject_edit BEFORE UPDATE ON library BEGIN SELECT RAISE(ABORT,'test disk/db failure'); END;")?;Ok(())}).await.unwrap();
    let mut edited = original.clone();
    edited.title = "New destination".into();
    let settings = Settings {
        output_dir: dir.path().join("library").to_string_lossy().into_owned(),
        file_template: "{title}".into(),
        auto_organize: true,
        ..Default::default()
    };
    let input = path.clone();
    assert!(db
        .call(move |conn| files::write(conn, &input, &edited, true, &settings))
        .await
        .is_err());
    assert_eq!(tagging::read_tags(&path).unwrap(), original);
    assert_eq!(
        std::fs::read_to_string(path.with_extension("lrc")).unwrap(),
        "original lyrics"
    );
    assert!(!dir.path().join("library/New destination.opus").exists());
    assert_eq!(page(&db).await.items[0].title, original.title);
}

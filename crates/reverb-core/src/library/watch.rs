//! Two-second event debounce followed by one second of stable size before probing.
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{RecursiveMode, Watcher};
use tokio_util::sync::CancellationToken;

use crate::{CoreResult, Db, EventSink, SettingsService};

pub type ProbeResolver = Arc<dyn Fn() -> CoreResult<PathBuf> + Send + Sync>;

pub async fn run(
    db: Db,
    settings: Arc<SettingsService>,
    sink: Arc<dyn EventSink>,
    resolve: ProbeResolver,
    cancel: CancellationToken,
) {
    match db.call(|conn| super::files::normalize_paths(conn)).await {
        Ok(count) if count > 0 => sink.emit("library://changed", serde_json::json!({})),
        Err(error) => tracing::warn!(%error,"Library path normalization failed"),
        _ => {}
    }
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut watcher =
        match notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            let _ = tx.send(event);
        }) {
            Ok(watcher) => watcher,
            Err(error) => {
                tracing::warn!(%error,"Library watcher unavailable");
                return;
            }
        };
    let mut watched: Option<PathBuf> = None;
    let mut pending: BTreeMap<PathBuf, (Instant, Option<u64>)> = BTreeMap::new();
    let mut tick = tokio::time::interval(Duration::from_millis(250));
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            event = rx.recv() => {
                match event {
                    Some(Ok(event)) => {
                        // Reads made by our own import must not generate another import.
                        if matches!(event.kind, notify::EventKind::Access(_)) { continue; }
                        for path in event.paths {
                            if super::files::supported(&path) {
                                pending.insert(path,(Instant::now()+Duration::from_secs(2),None));
                            }
                        }
                    }
                    Some(Err(error)) => tracing::warn!(%error,"Library watch event failed"),
                    None => break,
                }
            }
            _ = tick.tick() => {
                let config = settings.get();
                let root = crate::paths::resolve_output_dir(&config);
                let desired = (config.watch_library && root.is_dir()).then_some(root);
                if watched != desired {
                    if let Some(old) = watched.take() { let _ = watcher.unwatch(&old); }
                    pending.clear();
                    if let Some(root) = desired {
                        match watcher.watch(&root,RecursiveMode::Recursive) {
                            Ok(()) => watched = Some(root),
                            Err(error) => tracing::warn!(%error,"Cannot watch library folder"),
                        }
                    }
                }
                let ready: Vec<_> = pending.iter().filter(|(_, (due,_))| *due<=Instant::now()).map(|(path,_)|path.clone()).collect();
                for path in ready {
                    if crate::organize::is_moving(&path) {
                        pending.insert(path,(Instant::now()+Duration::from_secs(1),None));
                        continue;
                    }
                    let size = std::fs::metadata(&path).ok().filter(|m|m.is_file()).map(|m|m.len());
                    if let Some(size) = size {
                        if pending.get(&path).and_then(|(_,size)|*size) != Some(size) {
                            pending.insert(path,(Instant::now()+Duration::from_secs(1),Some(size)));
                            continue;
                        }
                        pending.remove(&path);
                        if let Ok(ffprobe)=resolve() {
                            if let Err(error)=super::files::import(&db,vec![path.clone()],&ffprobe,sink.clone(),"scan").await {
                                tracing::warn!(%error,"Library watch import failed");
                            }
                            if config.verify_lossless_on_import {
                                let ffmpeg=ffprobe.with_file_name(if cfg!(windows){"ffmpeg.exe"}else{"ffmpeg"});
                                match crate::lossless::verify_imports(&db,vec![path],&ffmpeg,&ffprobe).await {
                                    Ok(failures) if failures.is_empty()=>sink.emit("library://changed",serde_json::json!({})),
                                    Ok(_)=>tracing::warn!("Library watch spectrum verification failed"),
                                    Err(error)=>tracing::warn!(%error,"Library watch spectrum verification failed"),
                                }
                            }
                        }
                    } else {
                        pending.remove(&path);
                        let normalized=path.parent().and_then(|parent|super::files::canonical(parent).ok()).and_then(|parent|path.file_name().map(|name|parent.join(name))).unwrap_or(path);
                        let text = normalized.to_string_lossy().into_owned();
                        let changed = db.call(move |conn| Ok(conn.execute("UPDATE library SET missing=1,updated_at=unixepoch() WHERE file_path=? AND missing=0",[text])?)).await;
                        if matches!(changed,Ok(count) if count>0) { sink.emit("library://changed",serde_json::json!({})); }
                    }
                }
            }
        }
    }
}

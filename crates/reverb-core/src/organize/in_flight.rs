//! Publication paths are reserved until their database record is committed.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

fn registry() -> &'static Mutex<HashMap<PathBuf, usize>> {
    static PATHS: OnceLock<Mutex<HashMap<PathBuf, usize>>> = OnceLock::new();
    PATHS.get_or_init(Mutex::default)
}

fn key(path: &Path) -> PathBuf {
    let mut existing = path;
    let mut suffix = Vec::new();
    while !existing.exists() {
        if let Some(name) = existing.file_name() {
            suffix.push(name.to_os_string());
        }
        if let Some(parent) = existing.parent() {
            existing = parent;
        } else {
            break;
        }
    }
    let mut result = std::fs::canonicalize(existing).unwrap_or_else(|_| existing.into());
    for component in suffix.into_iter().rev() {
        result.push(component);
    }
    let text = result
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_owned();
    #[cfg(windows)]
    let text = text.to_lowercase();
    text.into()
}

#[derive(Debug, Default)]
pub struct MovingPaths(Vec<PathBuf>);
impl MovingPaths {
    pub fn reserve(&mut self, path: &Path) {
        let path = key(path);
        *registry().lock().unwrap().entry(path.clone()).or_default() += 1;
        self.0.push(path);
    }
}
impl Drop for MovingPaths {
    fn drop(&mut self) {
        let mut paths = registry().lock().unwrap();
        for path in &self.0 {
            if let Some(count) = paths.get_mut(path) {
                *count -= 1;
                if *count == 0 {
                    paths.remove(path);
                }
            }
        }
    }
}
pub fn is_moving(path: &Path) -> bool {
    registry().lock().unwrap().contains_key(&key(path))
}

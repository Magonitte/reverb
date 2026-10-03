//! Atomic replacement with rollback until the database transaction commits.
use crate::{CoreError, CoreResult};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Replacement {
    target: PathBuf,
    backup: Option<tempfile::NamedTempFile>,
    _moving: crate::organize::MovingPaths,
}
impl Replacement {
    pub fn publish(ready: &Path, target: &Path) -> CoreResult<Self> {
        let parent = target
            .parent()
            .ok_or_else(|| CoreError::invalid("Missing parent"))?;
        let backup = tempfile::Builder::new()
            .prefix(".reverb-old-")
            .suffix(&format!(
                ".{}",
                target.extension().unwrap_or_default().to_string_lossy()
            ))
            .tempfile_in(parent)?;
        std::fs::copy(target, backup.path())?;
        backup.as_file().sync_all()?;
        let staging = tempfile::Builder::new()
            .prefix(".reverb-new-")
            .tempfile_in(parent)?;
        std::fs::copy(ready, staging.path())?;
        staging.as_file().sync_all()?;
        let mut moving = crate::organize::MovingPaths::default();
        moving.reserve(target);
        staging
            .persist(target)
            .map_err(|e| CoreError::from(e.error))?;
        Ok(Self {
            target: target.to_owned(),
            backup: Some(backup),
            _moving: moving,
        })
    }
    pub fn commit(&mut self) -> CoreResult<()> {
        if let Some(backup) = self.backup.take() {
            let (_, path) = backup.keep().map_err(|e| CoreError::from(e.error))?;
            trash::delete(&path).map_err(|e| {
                CoreError::coded(
                    "disk",
                    format!("Old audio preserved at {}: {e}", path.display()),
                )
            })?;
        }
        Ok(())
    }
}
impl Drop for Replacement {
    fn drop(&mut self) {
        if let Some(backup) = self.backup.take() {
            if let Err(error) = backup.persist(&self.target) {
                tracing::error!(error=%error.error,"audio rollback failed; keeping original backup");
                let _ = error.file.keep();
            }
        }
    }
}

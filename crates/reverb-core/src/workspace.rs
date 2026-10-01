//! Pasta temporária de cada job: `<data>/tmp/<job_id>/`, apagada no `Drop` (arquitetura §8).

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::error::{CoreError, CoreResult};

pub struct JobWorkspace {
    dir: PathBuf,
}

impl JobWorkspace {
    /// Cria `<data_dir>/tmp/<job_id>/`. O `job_id` precisa ser um único componente de caminho.
    pub fn new(data_dir: &Path, job_id: &str) -> CoreResult<Self> {
        let valid = !job_id.is_empty()
            && job_id != "."
            && job_id != ".."
            && !job_id.contains(['/', '\\', ':', '\0']);
        if !valid {
            return Err(CoreError::invalid(format!("job_id inválido: {job_id:?}")));
        }
        let dir = data_dir.join("tmp").join(job_id);
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for JobWorkspace {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_dir_all(&self.dir) {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(dir = %self.dir.display(), error = %e, "não foi possível apagar a pasta do job");
            }
        }
    }
}

/// Apaga o que sobrou em `<data_dir>/tmp/` de execuções anteriores (entradas modificadas há mais
/// de `older_than`). Na inicialização do app use `Duration::ZERO`. Devolve quantas entradas removeu.
pub fn sweep_orphans(data_dir: &Path, older_than: Duration) -> usize {
    let Ok(entries) = std::fs::read_dir(data_dir.join("tmp")) else {
        return 0;
    };
    let now = SystemTime::now();
    let mut removed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let old_enough = entry
            .metadata()
            .and_then(|m| m.modified())
            .map(|modified| now.duration_since(modified).unwrap_or_default() >= older_than)
            .unwrap_or(true);
        if !old_enough {
            continue;
        }
        let result = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        match result {
            Ok(()) => removed += 1,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "falha ao limpar tmp órfão")
            }
        }
    }
    removed
}

#[cfg(test)]
mod tests;

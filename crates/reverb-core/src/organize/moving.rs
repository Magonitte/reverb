use std::fs::File;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use crate::{CoreError, CoreResult};

use super::template::check_path;
use super::unique_path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveMode {
    Auto,
    /// Exercita a publicação entre volumes sem depender da máquina ter dois volumes.
    Copy,
}

pub fn move_into_library(source: &Path, target: &Path) -> CoreResult<PathBuf> {
    move_into_library_with_mode(source, target, MoveMode::Auto)
}

fn rename_noclobber(source: &Path, target: &Path) -> io::Result<()> {
    let mut path = tempfile::TempPath::try_from_path(source)?;
    // A origem pertence ao chamador; uma tentativa malsucedida nunca deve apagá-la.
    path.disable_cleanup(true);
    path.persist_noclobber(target).map_err(|error| error.error)
}

/// Publicação sem sobrescrever: cópia entre volumes fica oculta até `sync_all` + rename.
pub fn move_into_library_with_mode(
    source: &Path,
    target: &Path,
    mode: MoveMode,
) -> CoreResult<PathBuf> {
    if source == target {
        return Err(CoreError::invalid("Origem e destino são o mesmo arquivo"));
    }
    if !std::fs::metadata(source)?.is_file() {
        return Err(CoreError::invalid("Origem não é um arquivo"));
    }
    check_path(target)?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut candidate = unique_path(target);
    if mode == MoveMode::Auto {
        loop {
            check_path(&candidate)?;
            match rename_noclobber(source, &candidate) {
                Ok(()) => return Ok(candidate),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                    candidate = unique_path(target)
                }
                Err(error) if error.kind() == ErrorKind::CrossesDevices => break,
                Err(error) => return Err(error.into()),
            }
        }
    }
    let mut input = File::open(source)?;
    let mut stage = tempfile::Builder::new()
        .prefix(".reverb-move-")
        .tempfile_in(parent)?;
    io::copy(&mut input, stage.as_file_mut())?;
    stage.as_file().sync_all()?;
    drop(input);
    loop {
        check_path(&candidate)?;
        match stage.persist_noclobber(&candidate) {
            Ok(file) => {
                drop(file);
                if let Err(error) = std::fs::remove_file(source) {
                    // Não deixar duas cópias após uma falha de remoção da origem.
                    std::fs::remove_file(&candidate)?;
                    return Err(error.into());
                }
                return Ok(candidate);
            }
            Err(error) if error.error.kind() == ErrorKind::AlreadyExists => {
                stage = error.file;
                candidate = unique_path(target);
            }
            Err(error) => return Err(error.error.into()),
        }
    }
}

#[cfg(test)]
mod tests;

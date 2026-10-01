//! Peças da instalação: download com progresso, extração, busca de binários e teste de fumaça
//! (arquitetura §16). O orquestrador fica em `manager.rs`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use super::process::run_capture;
use super::spec::PackageKind;
use crate::error::{CoreError, CoreResult};

/// Baixa `url` para `dest` chamando `on_percent` (0–100) a cada ponto percentual novo.
/// Devolve o SHA-256 (hex minúsculo) do que foi gravado.
pub async fn download_to(
    http: &reqwest::Client,
    url: &str,
    dest: &Path,
    mut on_percent: impl FnMut(u8),
) -> CoreResult<String> {
    let response = http.get(url).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(CoreError::coded(
            "download_failed",
            format!("o download respondeu HTTP {status}"),
        ));
    }
    let total = response.content_length().filter(|n| *n > 0);
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut file = tokio::fs::File::create(dest).await?;
    let mut hasher = Sha256::new();
    let mut downloaded: u64 = 0;
    let mut last_percent: i32 = -1;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        if let Some(total) = total {
            let percent = ((downloaded * 100) / total).min(100) as i32;
            if percent != last_percent {
                last_percent = percent;
                on_percent(percent as u8);
            }
        }
    }
    file.flush().await?;
    if total.is_none() {
        on_percent(100);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Baixa um arquivo pequeno de texto (checksums).
pub async fn fetch_text(http: &reqwest::Client, url: &str) -> CoreResult<String> {
    let response = http.get(url).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(CoreError::coded(
            "download_failed",
            format!("o download respondeu HTTP {status}"),
        ));
    }
    Ok(response.text().await?)
}

/// Extrai o pacote em `dest`. Zip pelo crate (ignora entradas com caminho inseguro);
/// tar pelo comando `tar` do sistema (arquitetura §16).
pub async fn extract(kind: PackageKind, archive: &Path, dest: &Path) -> CoreResult<()> {
    tokio::fs::create_dir_all(dest).await?;
    match kind {
        PackageKind::Exe => Err(CoreError::Internal("pacote exe não é extraído".into())),
        PackageKind::Zip => {
            let (archive, dest) = (archive.to_path_buf(), dest.to_path_buf());
            tokio::task::spawn_blocking(move || extract_zip(&archive, &dest))
                .await
                .map_err(|e| CoreError::Internal(format!("tarefa de extração falhou: {e}")))?
        }
        PackageKind::TarXz | PackageKind::TarGz => {
            let output = run_capture(
                "tar",
                [
                    std::ffi::OsStr::new("-xf"),
                    archive.as_os_str(),
                    std::ffi::OsStr::new("-C"),
                    dest.as_os_str(),
                ],
                |_| {},
                Duration::from_secs(300),
            )
            .await
            .map_err(|e| {
                CoreError::coded(
                    "extract_failed",
                    format!("não foi possível rodar o tar: {e}"),
                )
            })?;
            if output.success {
                Ok(())
            } else {
                Err(CoreError::coded(
                    "extract_failed",
                    format!("tar falhou: {}", output.stderr.trim()),
                ))
            }
        }
    }
}

pub fn extract_zip(archive: &Path, dest: &Path) -> CoreResult<()> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| CoreError::coded("extract_failed", format!("zip inválido: {e}")))?;
    zip.extract(dest)
        .map_err(|e| CoreError::coded("extract_failed", format!("falha ao extrair o zip: {e}")))
}

/// Procura um arquivo chamado `name` dentro de `root`; devolve o de menor profundidade.
pub fn find_file(root: &Path, name: &str) -> Option<PathBuf> {
    find_entry(root, name, false)
}

/// Procura uma pasta chamada `name` dentro de `root`; devolve a de menor profundidade.
pub fn find_dir(root: &Path, name: &str) -> Option<PathBuf> {
    find_entry(root, name, true)
}

fn find_entry(root: &Path, name: &str, want_dir: bool) -> Option<PathBuf> {
    let mut level = vec![root.to_path_buf()];
    while !level.is_empty() {
        let mut next = Vec::new();
        for dir in &level {
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            let mut entries: Vec<_> = entries.flatten().collect();
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                let path = entry.path();
                let is_dir = path.is_dir();
                if entry.file_name().to_string_lossy() == name && is_dir == want_dir {
                    return Some(path);
                }
                if is_dir {
                    next.push(path);
                }
            }
        }
        level = next;
    }
    None
}

pub fn copy_dir_all(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// `chmod 755` nos sistemas Unix; no Windows não faz nada.
pub fn make_executable(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

/// Renomeia com até 3 tentativas (antivírus pode segurar o arquivo recém-extraído).
pub async fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut last = None;
    for attempt in 0..3 {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) => last = Some(e),
        }
        if attempt < 2 {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
    Err(last.expect("ao menos uma tentativa"))
}

/// Executa `program args…` e devolve stdout + stderr. Tenta até 3× com 1 s de intervalo
/// (antivírus pode bloquear o executável recém-extraído) antes de falhar.
pub async fn smoke_test(
    program: &Path,
    args: &[&str],
    extra_env: &[(String, String)],
) -> CoreResult<String> {
    let mut last_error = String::new();
    for attempt in 0..3 {
        let result = run_capture(
            program,
            args,
            |command| {
                for (key, value) in extra_env {
                    command.env(key, value);
                }
            },
            Duration::from_secs(30),
        )
        .await;
        match result {
            Ok(output) if output.success => {
                return Ok(format!("{}\n{}", output.stdout, output.stderr));
            }
            Ok(output) => {
                last_error = format!(
                    "código {:?}: {}",
                    output.code,
                    output.stderr.lines().next().unwrap_or("").trim()
                );
            }
            Err(e) => last_error = e.to_string(),
        }
        if attempt < 2 {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
    Err(CoreError::coded(
        "smoke_test_failed",
        format!(
            "o teste de fumaça de {} falhou ({last_error})",
            program.display()
        ),
    ))
}

#[cfg(test)]
mod tests;

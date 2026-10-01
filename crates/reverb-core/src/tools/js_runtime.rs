//! Escolha do runtime JavaScript do yt-dlp (arquitetura §16, `jsRuntime`).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use regex::Regex;

use super::process::run_capture;
use crate::error::{CoreError, CoreResult};
use crate::settings::JsRuntime;

/// Versões mínimas aceitas dos runtimes do sistema.
pub const MIN_NODE_MAJOR: u64 = 20;
pub const MIN_DENO_MAJOR: u64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsKind {
    Deno,
    Node,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsRuntimeChoice {
    pub kind: JsKind,
    pub path: PathBuf,
}

impl JsRuntimeChoice {
    /// Valor de `--js-runtimes`: `deno:<caminho>` ou `node:<caminho>`.
    pub fn js_runtime_arg(&self) -> String {
        let kind = match self.kind {
            JsKind::Deno => "deno",
            JsKind::Node => "node",
        };
        format!("{kind}:{}", self.path.display())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Detection {
    Found(JsRuntimeChoice),
    /// Nada utilizável: o gerenciador precisa instalar o Deno gerenciado.
    NeedManagedDeno,
}

/// Procura `name` nos diretórios de `path_var` (como o `which`), sem tocar no ambiente do processo.
pub fn which_in(name: &str, path_var: &OsStr) -> Option<PathBuf> {
    let extensions: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat"]
    } else {
        &[""]
    };
    for dir in std::env::split_paths(path_var) {
        for ext in extensions {
            let candidate = dir.join(format!("{name}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn major_of(text: &str) -> Option<u64> {
    let re = Regex::new(r"(\d+)\.\d+\.\d+").expect("regex da versão");
    re.captures(text)?.get(1)?.as_str().parse().ok()
}

async fn version_output(program: &Path) -> Option<String> {
    let output = run_capture(program, ["--version"], |_| {}, Duration::from_secs(15))
        .await
        .ok()?;
    output.success.then_some(output.stdout)
}

/// Deno do sistema com versão suficiente (`deno --version` ⇒ `deno X.Y.Z …`).
async fn system_deno(path_var: &OsStr) -> Option<PathBuf> {
    let path = which_in("deno", path_var)?;
    let out = version_output(&path).await?;
    let line = out.lines().find(|l| l.trim_start().starts_with("deno "))?;
    (major_of(line)? >= MIN_DENO_MAJOR).then_some(path)
}

/// Node do sistema ≥ 20 (`node --version` ⇒ `vX.Y.Z`).
async fn system_node(path_var: &OsStr) -> Option<PathBuf> {
    let path = which_in("node", path_var)?;
    let out = version_output(&path).await?;
    (major_of(&out)? >= MIN_NODE_MAJOR).then_some(path)
}

/// Aplica a regra de `jsRuntime`. `path_var` vem do chamador (o app passa
/// `std::env::var_os("PATH")`; testes passam um PATH falso). `managed_deno` é o Deno gerenciado,
/// se já instalado.
pub async fn detect_js_runtime(
    setting: JsRuntime,
    path_var: &OsStr,
    managed_deno: Option<PathBuf>,
) -> CoreResult<Detection> {
    let deno = |path| JsRuntimeChoice {
        kind: JsKind::Deno,
        path,
    };
    let managed = |managed_deno: Option<PathBuf>| match managed_deno {
        Some(path) => Detection::Found(deno(path)),
        None => Detection::NeedManagedDeno,
    };
    match setting {
        JsRuntime::ManagedDeno => Ok(managed(managed_deno)),
        JsRuntime::SystemDeno => system_deno(path_var)
            .await
            .map(|path| Detection::Found(deno(path)))
            .ok_or_else(|| {
                CoreError::coded(
                    "js_runtime_missing",
                    format!("Deno ≥ {MIN_DENO_MAJOR} não encontrado no PATH"),
                )
            }),
        JsRuntime::SystemNode => system_node(path_var)
            .await
            .map(|path| {
                Detection::Found(JsRuntimeChoice {
                    kind: JsKind::Node,
                    path,
                })
            })
            .ok_or_else(|| {
                CoreError::coded(
                    "js_runtime_missing",
                    format!("Node ≥ {MIN_NODE_MAJOR} não encontrado no PATH"),
                )
            }),
        JsRuntime::Auto => {
            if let Some(path) = system_deno(path_var).await {
                return Ok(Detection::Found(deno(path)));
            }
            if let Some(path) = system_node(path_var).await {
                return Ok(Detection::Found(JsRuntimeChoice {
                    kind: JsKind::Node,
                    path,
                }));
            }
            Ok(managed(managed_deno))
        }
    }
}

#[cfg(test)]
mod tests;

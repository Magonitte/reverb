//! Sanitização de nomes **sempre com as regras do Windows**, também no Linux (arquitetura §13, regra 4).

use std::path::{Path, PathBuf};

use unicode_normalization::UnicodeNormalization;

pub const MAX_COMPONENT_CHARS: usize = 120;
pub const MAX_PATH_CHARS: usize = 240;

const RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

fn trim_edges(text: &str) -> &str {
    text.trim_start().trim_end_matches([' ', '.'])
}

/// Corta em `max` caracteres Unicode (nunca no meio de um) e limpa as pontas de novo.
fn truncate_chars(text: &str, max: usize) -> String {
    let cut: String = text.chars().take(max).collect();
    trim_edges(&cut).to_string()
}

fn is_reserved(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or("").trim_end();
    RESERVED.iter().any(|r| r.eq_ignore_ascii_case(stem))
}

/// Um componente de caminho seguro: sem caracteres proibidos, sem ponto/espaço final, sem nome
/// reservado, com até 120 caracteres. Nunca devolve vazio.
pub fn sanitize_component(input: &str) -> String {
    let mut replaced = String::with_capacity(input.len());
    for c in input.nfc() {
        match c {
            ':' => replaced.push_str(" -"),
            '<' | '>' | '"' | '/' | '\\' | '|' | '?' | '*' => replaced.push('_'),
            c if c.is_control() => replaced.push('_'),
            c => replaced.push(c),
        }
    }
    let mut name = truncate_chars(&replaced, MAX_COMPONENT_CHARS);
    if name.is_empty() {
        name.push('_');
    }
    if is_reserved(&name) {
        name.insert(0, '_');
    }
    name
}

fn sanitize_extension(ext: &str) -> String {
    ext.chars()
        .filter(char::is_ascii_alphanumeric)
        .take(8)
        .collect()
}

fn char_len(path: &Path) -> usize {
    path.to_string_lossy().chars().count()
}

/// Monta `base/<partes…>/<arquivo>.<ext>`: todas as partes são sanitizadas e a última é o nome do
/// arquivo (sem extensão). Se o caminho passar de 240 caracteres, encurta o nome do arquivo.
pub fn sanitize_path(base: &Path, parts: &[&str], ext: &str) -> PathBuf {
    let Some((file, dirs)) = parts.split_last() else {
        return base.to_path_buf();
    };
    let mut dir = base.to_path_buf();
    for part in dirs {
        dir.push(sanitize_component(part));
    }
    let ext = sanitize_extension(ext);
    let suffix = if ext.is_empty() {
        String::new()
    } else {
        format!(".{ext}")
    };

    let mut stem = sanitize_component(file);
    loop {
        let candidate = dir.join(format!("{stem}{suffix}"));
        let excess = char_len(&candidate).saturating_sub(MAX_PATH_CHARS);
        let stem_len = stem.chars().count();
        if excess == 0 || stem_len <= 1 {
            return candidate;
        }
        let shortened = truncate_chars(&stem, stem_len.saturating_sub(excess).max(1));
        stem = if shortened.is_empty() {
            "_".to_string()
        } else {
            shortened
        };
    }
}

/// Se `path` já existe, acrescenta ` (2)`, ` (3)`… antes da extensão até achar um nome livre.
pub fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned());
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    (2u32..)
        .map(|n| match &ext {
            Some(ext) => parent.join(format!("{stem} ({n}).{ext}")),
            None => parent.join(format!("{stem} ({n})")),
        })
        .find(|candidate| !candidate.exists())
        .expect("sempre existe um sufixo livre")
}

#[cfg(test)]
mod tests;

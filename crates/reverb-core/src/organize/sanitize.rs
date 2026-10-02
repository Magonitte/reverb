//! Sanitização de nomes **sempre com as regras do Windows**, também no Linux (arquitetura §13, regra 4).

use std::path::{Path, PathBuf};

use unicode_normalization::UnicodeNormalization;

pub const MAX_COMPONENT_CHARS: usize = 120;
pub const MAX_COMPONENT_BYTES: usize = 255;
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
    truncate_name(text, max, MAX_COMPONENT_BYTES)
}

fn truncate_name(text: &str, max_chars: usize, max_bytes: usize) -> String {
    let mut bytes = 0;
    let cut: String = text
        .chars()
        .take(max_chars)
        .take_while(|c| {
            bytes += c.len_utf8();
            bytes <= max_bytes
        })
        .collect();
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
        name = truncate_chars(&name, MAX_COMPONENT_CHARS);
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

    let mut stem = truncate_name(
        &sanitize_component(file),
        MAX_COMPONENT_CHARS - suffix.chars().count(),
        MAX_COMPONENT_BYTES - suffix.len(),
    );
    if stem.is_empty() {
        stem.push('_');
    }
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
        // Cortar pode transformar `CONcert` em `CON`, por exemplo.
        if is_reserved(&stem) {
            stem.insert(0, '_');
        }
    }
}

/// Se `path` já existe, acrescenta ` (2)`, ` (3)`… antes da extensão até achar um nome livre.
pub fn unique_path(path: &Path) -> PathBuf {
    unique_path_for(path, |candidate| {
        std::fs::symlink_metadata(candidate).is_ok()
    })
}

pub(crate) fn unique_path_for(path: &Path, occupied: impl Fn(&Path) -> bool) -> PathBuf {
    if !occupied(path) {
        return path.to_path_buf();
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned());
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    (2u32..)
        .map(|n| {
            let suffix = match &ext {
                Some(ext) => format!(" ({n}).{ext}"),
                None => format!(" ({n})"),
            };
            let limit =
                MAX_COMPONENT_CHARS.min(MAX_PATH_CHARS.saturating_sub(char_len(parent) + 1));
            let shortened = truncate_name(
                &stem,
                limit.saturating_sub(suffix.chars().count()).max(1),
                MAX_COMPONENT_BYTES.saturating_sub(suffix.len()),
            );
            parent.join(format!(
                "{}{suffix}",
                if shortened.is_empty() {
                    "_"
                } else {
                    &shortened
                }
            ))
        })
        .find(|candidate| !occupied(candidate))
        .expect("sempre existe um sufixo livre")
}

#[cfg(test)]
mod tests;

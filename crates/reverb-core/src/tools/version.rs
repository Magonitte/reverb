//! Extração e comparação de versões das ferramentas (arquitetura §16).

use std::cmp::Ordering;

use regex::Regex;

use super::spec::{Tool, ToolSpec, VersionKind};

/// Extrai a versão da saída de `--version` / `-version`.
pub fn parse_version(spec: &ToolSpec, output: &str) -> Option<String> {
    let re = Regex::new(spec.version_regex).ok()?;
    re.captures(output)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

/// Normaliza a `tag_name` do release para o formato da versão reportada pelo binário.
pub fn tag_to_version(tool: Tool, tag: &str) -> String {
    match tool {
        Tool::Deno | Tool::Fpcalc => tag.trim().trim_start_matches('v').to_string(),
        _ => tag.trim().to_string(),
    }
}

fn numeric_parts(text: &str) -> Vec<u64> {
    text.trim()
        .trim_start_matches('v')
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse().ok())
        .collect()
}

/// Compara duas versões do mesmo tipo. `Rolling` compara texto (timestamps ISO-8601 em UTC).
pub fn compare(kind: VersionKind, a: &str, b: &str) -> Ordering {
    match kind {
        VersionKind::Date => numeric_parts(a).cmp(&numeric_parts(b)),
        VersionKind::Semver => {
            let parse = |text: &str| semver::Version::parse(text.trim().trim_start_matches('v'));
            match (parse(a), parse(b)) {
                (Ok(x), Ok(y)) => x.cmp(&y),
                _ => numeric_parts(a).cmp(&numeric_parts(b)),
            }
        }
        VersionKind::Rolling => a.trim().cmp(b.trim()),
    }
}

#[cfg(test)]
mod tests;

//! Parsers de arquivos de checksum e hash de arquivos (arquitetura §16).

use std::io::Read;
use std::path::Path;

use regex::Regex;
use sha2::{Digest, Sha256};

fn is_sha256(token: &str) -> bool {
    token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Formato `<hash>  <nome>` (também `<hash> *<nome>`): devolve o hash (minúsculo) do arquivo.
pub fn parse_sums_for(text: &str, asset_name: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let mut tokens = line.split_whitespace();
        let hash = tokens.next()?;
        let name = tokens.next()?.trim_start_matches('*');
        (is_sha256(hash) && name == asset_name).then(|| hash.to_ascii_lowercase())
    })
}

/// Formato livre (um arquivo por asset): o primeiro token de 64 hex, em qualquer posição.
/// O Deno publica tanto `<hash>  <nome>` quanto a saída do `Get-FileHash` do PowerShell
/// (hash em maiúsculas), então a comparação é sempre em minúsculas.
pub fn parse_first_hash(text: &str) -> Option<String> {
    let re = Regex::new(r"\b[0-9a-fA-F]{64}\b").expect("regex do hash");
    re.find(text).map(|m| m.as_str().to_ascii_lowercase())
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests;

//! T2: parsers de checksum com os três arquivos reais gravados.

use super::*;
use crate::tools::testutil::fixture_text;

#[test]
fn ytdlp_sha2_256sums() {
    let text = fixture_text("checksums/ytdlp-SHA2-256SUMS");
    assert_eq!(
        parse_sums_for(&text, "yt-dlp.exe").unwrap(),
        "66674953fe251b89f4d08c5f0e35e0728679bd67ab3d7d05c0562af101dd3e7a"
    );
    assert_eq!(
        parse_sums_for(&text, "yt-dlp_linux").unwrap(),
        "58162f9bfdc27458ea47bfcb311cf47028f17d8154a8bf7d689861d46399230a"
    );
    // `yt-dlp` não pode casar com `yt-dlp.exe` por prefixo.
    assert_eq!(
        parse_sums_for(&text, "yt-dlp").unwrap(),
        "1fa6733c37ea6fb51c99ad8fe785e7b7e5f3246c9b980230329d4fb72ed8d4d6"
    );
    assert_eq!(parse_sums_for(&text, "nao-existe.bin"), None);
}

#[test]
fn ffmpeg_checksums_sha256() {
    let text = fixture_text("checksums/ffmpeg-checksums.sha256");
    assert_eq!(
        parse_sums_for(&text, "ffmpeg-master-latest-win64-gpl.zip").unwrap(),
        "ab9caf1306aeccb8f931b7c5916ba71b649359a55b157694c0c26d7fab02dfc7"
    );
    assert_eq!(
        parse_sums_for(&text, "ffmpeg-master-latest-linux64-gpl.tar.xz").unwrap(),
        "935b5de1817483a239c585c6702013fe4e886578bb83c3ef22a9d100305da68d"
    );
    // A variante `-shared` é outro arquivo, com outro hash.
    assert_ne!(
        parse_sums_for(&text, "ffmpeg-master-latest-win64-gpl-shared.zip").unwrap(),
        parse_sums_for(&text, "ffmpeg-master-latest-win64-gpl.zip").unwrap()
    );
}

#[test]
fn deno_nos_dois_formatos_reais() {
    // Windows: saída do Get-FileHash do PowerShell (hash em MAIÚSCULAS, várias linhas).
    let windows = fixture_text("checksums/deno-x86_64-pc-windows-msvc.zip.sha256sum");
    assert_eq!(
        parse_first_hash(&windows).unwrap(),
        "a0c3101b4158d1dfb7d6a78a7bf0f3de80c96bb423c152beec8beb22786f2238"
    );
    // Linux: `<hash>  <nome>`.
    let linux = fixture_text("checksums/deno-x86_64-unknown-linux-gnu.zip.sha256sum");
    assert_eq!(
        parse_first_hash(&linux).unwrap(),
        "c6527f24f4b16031d3ae4fa9f658d5f11534c8d84ce7dc8502420280919c3490"
    );
}

#[test]
fn formatos_alternativos_e_lixo() {
    let hash = "a".repeat(64);
    // Marcador de modo binário `*nome`.
    assert_eq!(
        parse_sums_for(&format!("{hash} *arquivo.zip\n"), "arquivo.zip").unwrap(),
        hash
    );
    // Linhas CRLF e em branco.
    assert_eq!(
        parse_sums_for(&format!("\r\n{hash}  arquivo.zip\r\n"), "arquivo.zip").unwrap(),
        hash
    );
    // Hash com tamanho errado não vale.
    assert_eq!(parse_sums_for("abc123  arquivo.zip", "arquivo.zip"), None);
    assert_eq!(parse_first_hash("sem hash aqui"), None);
    assert_eq!(parse_first_hash(&format!("{}x", "b".repeat(65))), None);
}

#[test]
fn sha256_de_arquivo() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("abc.txt");
    std::fs::write(&file, "abc").unwrap();
    assert_eq!(
        sha256_file(&file).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert!(sha256_file(&dir.path().join("nao-existe")).is_err());
}

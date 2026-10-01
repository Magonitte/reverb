//! F03 — validação de argumentos dos comandos de mídia (offline: falham antes de qualquer rede).

use std::process::{Command, Output};

fn cli(args: &[&str]) -> Output {
    let temp = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_reverb-cli"))
        .arg("--data-dir")
        .arg(temp.path().join("dados"))
        .arg("--tools-dir")
        .arg(temp.path().join("ferramentas"))
        .args(args)
        .output()
        .unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn perfil_desconhecido_lista_os_validos() {
    let output = cli(&[
        "download",
        "https://youtu.be/dQw4w9WgXcQ",
        "--profile",
        "wav",
        "--out",
        "x",
    ]);
    assert!(!output.status.success());
    let text = stderr(&output);
    assert!(text.contains("perfil desconhecido: wav"), "{text}");
    assert!(text.contains("mp3_v0") && text.contains("flac"), "{text}");
}

#[test]
fn download_rejeita_colecao_e_texto() {
    for url in [
        "https://www.youtube.com/playlist?list=PL123",
        "rick astley",
        "https://vimeo.com/1",
    ] {
        let output = cli(&["download", url, "--out", "x"]);
        assert!(!output.status.success(), "{url}");
        assert!(
            stderr(&output).contains("apenas a URL de um vídeo"),
            "{url}"
        );
    }
}

#[test]
fn analyze_rejeita_url_nao_suportada() {
    let output = cli(&["analyze", "https://vimeo.com/123"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("URL não suportada"));
}

#[test]
fn search_rejeita_fonte_desconhecida() {
    let output = cli(&["search", "x", "--source", "spotify"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("fonte desconhecida: spotify"));
}

#[test]
fn download_sem_ferramentas_pede_a_instalacao() {
    let output = cli(&["download", "https://youtu.be/dQw4w9WgXcQ", "--out", "x"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("tools install"),
        "{}",
        stderr(&output)
    );
}

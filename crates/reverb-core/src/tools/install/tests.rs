use std::sync::{Arc, Mutex};

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::tools::testutil::{copy_fake_tool, make_zip, sha256_hex};

#[test]
fn find_file_devolve_o_menos_profundo() {
    let dir = tempfile::tempdir().unwrap();
    let deep = dir.path().join("a").join("b").join("c");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("alvo.bin"), "fundo").unwrap();
    let shallow = dir.path().join("x");
    std::fs::create_dir_all(&shallow).unwrap();
    std::fs::write(shallow.join("alvo.bin"), "raso").unwrap();

    let found = find_file(dir.path(), "alvo.bin").unwrap();
    assert_eq!(std::fs::read_to_string(found).unwrap(), "raso");
    assert_eq!(find_file(dir.path(), "outro.bin"), None);
    // Uma pasta com o mesmo nome não conta como arquivo, e vice-versa.
    assert_eq!(find_file(dir.path(), "a"), None);
    assert!(find_dir(dir.path(), "b").is_some());
    assert_eq!(find_dir(dir.path(), "alvo.bin"), None);
}

#[tokio::test]
async fn extrai_zip_com_pastas_aninhadas() {
    let dir = tempfile::tempdir().unwrap();
    let zip_path = dir.path().join("pacote.zip");
    std::fs::write(
        &zip_path,
        make_zip(&[
            ("ffmpeg-master-latest-win64-gpl/bin/ffmpeg.exe", b"ff"),
            ("ffmpeg-master-latest-win64-gpl/bin/ffprobe.exe", b"fp"),
            ("ffmpeg-master-latest-win64-gpl/LICENSE.txt", b"licenca"),
        ]),
    )
    .unwrap();
    let out = dir.path().join("saida");
    extract(PackageKind::Zip, &zip_path, &out).await.unwrap();

    // Localiza pelo nome do binário, não por caminho fixo (armadilha do plano).
    let ffmpeg = find_file(&out, "ffmpeg.exe").unwrap();
    assert_eq!(std::fs::read(ffmpeg).unwrap(), b"ff");
    assert!(find_file(&out, "ffprobe.exe").is_some());
}

#[tokio::test]
async fn zip_com_caminho_malicioso_nao_escapa_do_destino() {
    let dir = tempfile::tempdir().unwrap();
    let zip_path = dir.path().join("mal.zip");
    std::fs::write(
        &zip_path,
        make_zip(&[("../fora.txt", b"escapou"), ("ok.txt", b"ok")]),
    )
    .unwrap();
    let out = dir.path().join("dentro").join("saida");
    let _ = extract(PackageKind::Zip, &zip_path, &out).await;

    assert!(!dir.path().join("dentro").join("fora.txt").exists());
    assert!(!dir.path().join("fora.txt").exists());
}

#[tokio::test]
async fn zip_invalido_vira_erro_extract_failed() {
    let dir = tempfile::tempdir().unwrap();
    let zip_path = dir.path().join("lixo.zip");
    std::fs::write(&zip_path, "isto não é um zip").unwrap();
    let err = extract(PackageKind::Zip, &zip_path, &dir.path().join("o"))
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "extract_failed");
}

#[tokio::test]
async fn download_reporta_progresso_e_devolve_o_hash() {
    let server = MockServer::start().await;
    let body = vec![7u8; 300_000];
    Mock::given(method("GET"))
        .and(path("/arquivo.bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("sub").join("arquivo.bin");

    let percents = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&percents);
    let hash = download_to(
        &reqwest::Client::new(),
        &format!("{}/arquivo.bin", server.uri()),
        &dest,
        move |p| seen.lock().unwrap().push(p),
    )
    .await
    .unwrap();

    assert_eq!(hash, sha256_hex(&body));
    assert_eq!(std::fs::read(&dest).unwrap(), body);
    let percents = percents.lock().unwrap();
    assert_eq!(percents.last(), Some(&100));
    assert!(percents.windows(2).all(|w| w[0] < w[1]), "{percents:?}");
}

#[tokio::test]
async fn download_com_http_de_erro_falha() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let err = download_to(
        &reqwest::Client::new(),
        &format!("{}/x", server.uri()),
        &dir.path().join("x"),
        |_| {},
    )
    .await
    .unwrap_err();
    assert_eq!(err.kind(), "download_failed");
    assert_eq!(
        fetch_text(&reqwest::Client::new(), &format!("{}/x", server.uri()))
            .await
            .unwrap_err()
            .kind(),
        "download_failed"
    );
}

#[tokio::test]
async fn teste_de_fumaca_le_a_versao_do_binario() {
    let dir = tempfile::tempdir().unwrap();
    let tool = dir.path().join(format!(
        "pasta com espaço/ferramenta{}",
        std::env::consts::EXE_SUFFIX
    ));
    copy_fake_tool(&tool);
    let out = smoke_test(
        &tool,
        &["--version"],
        &[("FAKE_TOOL_VERSION".to_string(), "2026.10.01".to_string())],
    )
    .await
    .unwrap();
    assert!(out.contains("2026.10.01"));
}

#[tokio::test]
async fn teste_de_fumaca_falha_depois_de_3_tentativas() {
    let dir = tempfile::tempdir().unwrap();
    let tool = dir
        .path()
        .join(format!("quebrada{}", std::env::consts::EXE_SUFFIX));
    copy_fake_tool(&tool);
    let started = std::time::Instant::now();
    let err = smoke_test(&tool, &["--exit", "3"], &[]).await.unwrap_err();
    assert_eq!(err.kind(), "smoke_test_failed");
    // 3 tentativas com 1 s entre elas (antivírus pode segurar o arquivo recém-extraído).
    assert!(started.elapsed() >= Duration::from_secs(2));

    let missing = smoke_test(&dir.path().join("nao-existe"), &["--version"], &[])
        .await
        .unwrap_err();
    assert_eq!(missing.kind(), "smoke_test_failed");
}

#[tokio::test]
async fn copia_pastas_e_renomeia_com_retry() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("origem");
    std::fs::create_dir_all(from.join("a/b")).unwrap();
    std::fs::write(from.join("a/b/x.txt"), "x").unwrap();
    copy_dir_all(&from, &dir.path().join("copia")).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("copia/a/b/x.txt")).unwrap(),
        "x"
    );

    rename_with_retry(&from, &dir.path().join("movida"))
        .await
        .unwrap();
    assert!(dir.path().join("movida/a/b/x.txt").is_file());
    assert!(!from.exists());
}

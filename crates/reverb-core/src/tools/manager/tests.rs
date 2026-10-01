//! T4–T7: instalação, checksum, atualização/rollback e lock — tudo offline (GitHub falso).

use std::ffi::OsString;
use std::time::Instant;

use serde_json::{json, Value};

use super::*;
use crate::settings::{SettingsPatch, YtdlpChannel};
use crate::tools::testutil::{fake_tool_bytes, make_zip, sha256_hex, FakeGithub, Harness};

/// O `fake-tool` imprime tudo isto: cada ferramenta lê a sua linha (regex da spec).
const UNIVERSAL_VERSION: &str = "2026.10.01\ndeno 2.9.8 (stable, release, x86_64-pc-windows-msvc)\nffmpeg version N-1234-gabcdef-20261001 Copyright (c) the FFmpeg developers\nfpcalc version 1.6.1";

const ENV: &[(&str, &str)] = &[("FAKE_TOOL_VERSION", UNIVERSAL_VERSION)];

async fn harness() -> Harness {
    Harness::new(ENV).await
}

async fn publish_ytdlp(gh: &FakeGithub, repo: &str, tag: &str) {
    let exe = fake_tool_bytes();
    let sums = format!(
        "{}  yt-dlp.exe\n{}  yt-dlp_linux\n",
        sha256_hex(&exe),
        "0".repeat(64)
    );
    gh.publish(
        repo,
        tag,
        "2026-10-01T00:00:00Z",
        vec![
            ("yt-dlp.exe".into(), exe),
            ("yt-dlp_linux".into(), b"linux".to_vec()),
            ("SHA2-256SUMS".into(), sums.into_bytes()),
        ],
    )
    .await;
}

async fn publish_deno(gh: &FakeGithub, tag: &str) {
    // O binário fica numa subpasta para provar a busca por nome.
    let zip = make_zip(&[
        ("pacote/deno.exe", &fake_tool_bytes()),
        ("pacote/LICENSE", b"mit"),
    ]);
    // Formato do Get-FileHash do PowerShell, com o hash em MAIÚSCULAS (como o Deno publica).
    let checksum = format!(
        "\r\nAlgorithm : SHA256\r\nHash      : {}\r\nPath      : C:\\a\\deno.zip\r\n\r\n",
        sha256_hex(&zip).to_ascii_uppercase()
    );
    gh.publish(
        "denoland/deno",
        tag,
        "2026-10-01T00:00:00Z",
        vec![
            ("deno-x86_64-pc-windows-msvc.zip".into(), zip),
            (
                "deno-x86_64-pc-windows-msvc.zip.sha256sum".into(),
                checksum.into_bytes(),
            ),
        ],
    )
    .await;
}

async fn publish_ffmpeg(gh: &FakeGithub, updated_at: &str) {
    let exe = fake_tool_bytes();
    let zip = make_zip(&[
        ("ffmpeg-master-latest-win64-gpl/bin/ffmpeg.exe", &exe),
        ("ffmpeg-master-latest-win64-gpl/bin/ffprobe.exe", &exe),
    ]);
    let sums = format!("{}  ffmpeg-master-latest-win64-gpl.zip\n", sha256_hex(&zip));
    gh.publish(
        "yt-dlp/FFmpeg-Builds",
        "latest",
        updated_at,
        vec![
            ("ffmpeg-master-latest-win64-gpl.zip".into(), zip),
            ("checksums.sha256".into(), sums.into_bytes()),
        ],
    )
    .await;
}

fn manifest_json(h: &Harness) -> Value {
    let text = std::fs::read_to_string(h.tools_dir().join("manifest.json")).unwrap_or("{}".into());
    serde_json::from_str(&text).unwrap()
}

fn current_version(h: &Harness, tool: &str) -> Option<String> {
    manifest_json(h)["tools"][tool]["current"]["version"]
        .as_str()
        .map(str::to_string)
}

fn previous_version(h: &Harness, tool: &str) -> Option<String> {
    manifest_json(h)["tools"][tool]["previous"]["version"]
        .as_str()
        .map(str::to_string)
}

fn version_dirs(h: &Harness, tool: &str) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(h.tools_dir().join(tool))
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn staging_is_clean(h: &Harness) -> bool {
    std::fs::read_dir(h.tools_dir().join(".staging"))
        .map(|rd| rd.count() == 0)
        .unwrap_or(true)
}

fn phases(h: &Harness, tool: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for event in h.sink.named(EVENT_PROGRESS) {
        if event["tool"] == tool {
            let phase = event["phase"].as_str().unwrap().to_string();
            if out.last() != Some(&phase) {
                out.push(phase);
            }
        }
    }
    out
}

// ------------------------------------------------------------------ T4

#[tokio::test]
async fn t4_instala_ytdlp_exe_solto_com_checksum() {
    let h = harness().await;
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.01").await;

    assert_eq!(
        h.manager.resolve(Tool::Ytdlp).unwrap_err().kind(),
        "tool_missing"
    );
    let outcome = h.manager.install(Tool::Ytdlp).await.unwrap();
    assert_eq!(
        outcome,
        InstallOutcome::Installed {
            version: "2026.10.01".into(),
            previous: None
        }
    );

    let path = h.manager.resolve(Tool::Ytdlp).unwrap();
    assert!(path.is_file());
    assert_eq!(path.file_name().unwrap(), "yt-dlp.exe");
    assert_eq!(current_version(&h, "ytdlp").as_deref(), Some("2026.10.01"));

    // O teste de fumaça leu a versão reportada pelo binário (FAKE_TOOL_VERSION).
    let manifest = manifest_json(&h);
    assert_eq!(
        manifest["tools"]["ytdlp"]["current"]["reported_version"],
        "2026.10.01"
    );
    assert_eq!(manifest["tools"]["ytdlp"]["current"]["channel"], "stable");

    let status = h.manager.status().await.unwrap();
    let ytdlp = status.iter().find(|s| s.tool == Tool::Ytdlp).unwrap();
    assert!(ytdlp.installed && ytdlp.required);
    assert_eq!(ytdlp.version.as_deref(), Some("2026.10.01"));
    assert!(
        !status
            .iter()
            .find(|s| s.tool == Tool::Deno)
            .unwrap()
            .installed
    );

    assert_eq!(
        phases(&h, "ytdlp"),
        ["downloading", "verifying", "extracting", "testing"]
    );
    assert_eq!(h.sink.named(EVENT_CHANGED).len(), 1);
    assert_eq!(h.sink.named(EVENT_CHANGED)[0]["tool"], "ytdlp");
    assert!(staging_is_clean(&h));
}

#[tokio::test]
async fn t4_instala_deno_de_um_zip_com_hash_em_maiusculas() {
    let h = harness().await;
    publish_deno(&h.github, "v2.9.8").await;

    h.manager.install(Tool::Deno).await.unwrap();
    let path = h.manager.resolve(Tool::Deno).unwrap();
    assert!(path.is_file());
    // Rótulo da versão = tag sem o `v`; a versão reportada vem do binário.
    assert_eq!(current_version(&h, "deno").as_deref(), Some("2.9.8"));
    assert_eq!(
        h.manager.status().await.unwrap()[1].version.as_deref(),
        Some("2.9.8")
    );
    assert!(staging_is_clean(&h));
}

#[tokio::test]
async fn t4_instala_ffmpeg_e_ffprobe_do_mesmo_pacote() {
    let h = harness().await;
    publish_ffmpeg(&h.github, "2026-09-30T19:00:56Z").await;

    h.manager.install(Tool::Ffmpeg).await.unwrap();
    assert!(h.manager.resolve(Tool::Ffmpeg).unwrap().is_file());
    assert!(h.manager.resolve_ffprobe().unwrap().is_file());
    // Tag rolante `latest`: a pasta da versão vem do `updated_at` (sem `:` por causa do Windows).
    assert_eq!(
        current_version(&h, "ffmpeg").as_deref(),
        Some("20260930T190056Z")
    );
}

#[tokio::test]
async fn instalar_de_novo_sem_mudanca_nao_baixa_nada() {
    let h = harness().await;
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.01").await;
    h.manager.install(Tool::Ytdlp).await.unwrap();
    let requests_before = h.github.server.received_requests().await.unwrap().len();

    let outcome = h.manager.update(Tool::Ytdlp).await.unwrap();
    assert_eq!(
        outcome,
        InstallOutcome::UpToDate {
            version: "2026.10.01".into()
        }
    );
    let requests = h.github.server.received_requests().await.unwrap();
    // Só a consulta da API (sem baixar o binário nem o checksum).
    assert_eq!(requests.len(), requests_before + 1);
    assert!(requests
        .last()
        .unwrap()
        .url
        .path()
        .ends_with("/releases/latest"));
    assert_eq!(h.sink.named(EVENT_CHANGED).len(), 1);
}

#[tokio::test]
async fn release_sem_o_asset_da_plataforma_da_erro_claro() {
    let h = harness().await;
    h.github
        .publish(
            "yt-dlp/yt-dlp",
            "2026.10.01",
            "2026-10-01T00:00:00Z",
            vec![("yt-dlp_macos".into(), b"x".to_vec())],
        )
        .await;
    let err = h.manager.install(Tool::Ytdlp).await.unwrap_err();
    assert_eq!(err.kind(), "asset_not_found");
    assert!(h.manager.resolve(Tool::Ytdlp).is_err());
}

#[tokio::test]
async fn pacote_sem_o_binario_esperado_nao_instala() {
    let h = harness().await;
    let zip = make_zip(&[("pacote/outra-coisa.exe", b"x")]);
    let checksum = sha256_hex(&zip);
    h.github
        .publish(
            "denoland/deno",
            "v2.9.8",
            "2026-10-01T00:00:00Z",
            vec![
                ("deno-x86_64-pc-windows-msvc.zip".into(), zip),
                (
                    "deno-x86_64-pc-windows-msvc.zip.sha256sum".into(),
                    format!("{checksum}  deno-x86_64-pc-windows-msvc.zip").into_bytes(),
                ),
            ],
        )
        .await;
    let err = h.manager.install(Tool::Deno).await.unwrap_err();
    assert_eq!(err.kind(), "binary_not_found");
    assert!(current_version(&h, "deno").is_none());
    assert!(staging_is_clean(&h));
}

#[tokio::test]
async fn binario_que_nao_passa_no_teste_de_fumaca_nao_e_instalado() {
    // O binário "responde" com um texto sem versão reconhecível.
    let h = Harness::new(&[("FAKE_TOOL_VERSION", "nada de versão aqui")]).await;
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.01").await;
    let err = h.manager.install(Tool::Ytdlp).await.unwrap_err();
    assert_eq!(err.kind(), "smoke_test_failed");
    assert!(current_version(&h, "ytdlp").is_none());
    assert!(version_dirs(&h, "ytdlp").is_empty());
}

// ------------------------------------------------------------------ T5

#[tokio::test]
async fn t5_checksum_errado_da_erro_e_nao_altera_o_manifesto() {
    let h = harness().await;
    let exe = fake_tool_bytes();
    h.github
        .publish(
            "yt-dlp/yt-dlp",
            "2026.10.01",
            "2026-10-01T00:00:00Z",
            vec![
                ("yt-dlp.exe".into(), exe),
                (
                    "SHA2-256SUMS".into(),
                    format!("{}  yt-dlp.exe\n", "f".repeat(64)).into_bytes(),
                ),
            ],
        )
        .await;

    let err = h.manager.install(Tool::Ytdlp).await.unwrap_err();
    assert_eq!(err.kind(), "checksum_mismatch");
    assert!(!h.tools_dir().join("manifest.json").exists());
    assert!(version_dirs(&h, "ytdlp").is_empty());
    assert!(h.manager.resolve(Tool::Ytdlp).is_err());
    assert!(staging_is_clean(&h));
    assert!(h.sink.named(EVENT_CHANGED).is_empty());
}

#[tokio::test]
async fn t5_checksum_errado_numa_atualizacao_mantem_a_versao_atual() {
    let h = harness().await;
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.01").await;
    h.manager.install(Tool::Ytdlp).await.unwrap();
    let before = manifest_json(&h);

    h.github
        .publish(
            "yt-dlp/yt-dlp",
            "2026.10.02",
            "2026-10-02T00:00:00Z",
            vec![
                ("yt-dlp.exe".into(), fake_tool_bytes()),
                (
                    "SHA2-256SUMS".into(),
                    format!("{}  yt-dlp.exe\n", "e".repeat(64)).into_bytes(),
                ),
            ],
        )
        .await;
    let err = h.manager.update(Tool::Ytdlp).await.unwrap_err();
    assert_eq!(err.kind(), "checksum_mismatch");
    assert_eq!(manifest_json(&h), before);
    assert_eq!(version_dirs(&h, "ytdlp"), ["2026.10.01"]);
    assert!(h.manager.resolve(Tool::Ytdlp).unwrap().is_file());
}

#[tokio::test]
async fn release_sem_arquivo_de_checksum_e_recusado() {
    let h = harness().await;
    h.github
        .publish(
            "yt-dlp/yt-dlp",
            "2026.10.01",
            "2026-10-01T00:00:00Z",
            vec![("yt-dlp.exe".into(), fake_tool_bytes())],
        )
        .await;
    let err = h.manager.install(Tool::Ytdlp).await.unwrap_err();
    assert_eq!(err.kind(), "checksum_missing");
    assert!(h.manager.resolve(Tool::Ytdlp).is_err());
}

#[tokio::test]
async fn fonte_sem_checksum_instala_com_aviso() {
    // fpcalc não publica checksum (§16): instala mesmo assim.
    let h = harness().await;
    let zip = make_zip(&[(
        "chromaprint-fpcalc-1.6.1-windows-x86_64/fpcalc.exe",
        &fake_tool_bytes(),
    )]);
    h.github
        .publish(
            "acoustid/chromaprint",
            "v1.6.1",
            "2026-07-28T06:18:09Z",
            vec![("chromaprint-fpcalc-1.6.1-windows-x86_64.zip".into(), zip)],
        )
        .await;
    h.manager.install(Tool::Fpcalc).await.unwrap();
    assert!(h.manager.resolve(Tool::Fpcalc).unwrap().is_file());
    assert_eq!(current_version(&h, "fpcalc").as_deref(), Some("1.6.1"));
}

// ------------------------------------------------------------------ T6

#[tokio::test]
async fn t6_atualiza_para_a_versao_2_e_faz_rollback() {
    let h = harness().await;
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.01").await;
    h.manager.install(Tool::Ytdlp).await.unwrap();

    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.02").await;
    let outcome = h.manager.update(Tool::Ytdlp).await.unwrap();
    assert_eq!(
        outcome,
        InstallOutcome::Installed {
            version: "2026.10.02".into(),
            previous: Some("2026.10.01".into())
        }
    );
    assert_eq!(current_version(&h, "ytdlp").as_deref(), Some("2026.10.02"));
    assert_eq!(previous_version(&h, "ytdlp").as_deref(), Some("2026.10.01"));
    assert_eq!(
        h.manager
            .resolve(Tool::Ytdlp)
            .unwrap()
            .parent()
            .unwrap()
            .file_name()
            .unwrap(),
        "2026.10.02"
    );

    let back = h.manager.rollback(Tool::Ytdlp).await.unwrap();
    assert_eq!(back, "2026.10.01");
    assert_eq!(current_version(&h, "ytdlp").as_deref(), Some("2026.10.01"));
    assert_eq!(previous_version(&h, "ytdlp").as_deref(), Some("2026.10.02"));
    assert_eq!(
        h.manager
            .resolve(Tool::Ytdlp)
            .unwrap()
            .parent()
            .unwrap()
            .file_name()
            .unwrap(),
        "2026.10.01"
    );
    // Rollback emitiu `tools://changed` (instalação 1, atualização, rollback).
    assert_eq!(h.sink.named(EVENT_CHANGED).len(), 3);

    // E de novo: current ↔ previous.
    assert_eq!(h.manager.rollback(Tool::Ytdlp).await.unwrap(), "2026.10.02");
}

#[tokio::test]
async fn t6_so_duas_versoes_ficam_apos_a_terceira_instalacao() {
    let h = harness().await;
    for tag in ["2026.10.01", "2026.10.02", "2026.10.03"] {
        publish_ytdlp(&h.github, "yt-dlp/yt-dlp", tag).await;
        h.manager.update(Tool::Ytdlp).await.unwrap();
    }
    assert_eq!(version_dirs(&h, "ytdlp"), ["2026.10.02", "2026.10.03"]);
    assert_eq!(current_version(&h, "ytdlp").as_deref(), Some("2026.10.03"));
    assert_eq!(previous_version(&h, "ytdlp").as_deref(), Some("2026.10.02"));
}

#[tokio::test]
async fn rollback_apos_rollback_e_nova_instalacao_mantem_duas_versoes() {
    let h = harness().await;
    for tag in ["2026.10.01", "2026.10.02"] {
        publish_ytdlp(&h.github, "yt-dlp/yt-dlp", tag).await;
        h.manager.update(Tool::Ytdlp).await.unwrap();
    }
    h.manager.rollback(Tool::Ytdlp).await.unwrap(); // current = .01, previous = .02
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.03").await;
    h.manager.update(Tool::Ytdlp).await.unwrap();
    assert_eq!(version_dirs(&h, "ytdlp"), ["2026.10.01", "2026.10.03"]);
    assert_eq!(previous_version(&h, "ytdlp").as_deref(), Some("2026.10.01"));
}

#[tokio::test]
async fn rollback_sem_versao_anterior_da_erro() {
    let h = harness().await;
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.01").await;
    h.manager.install(Tool::Ytdlp).await.unwrap();
    let err = h.manager.rollback(Tool::Ytdlp).await.unwrap_err();
    assert_eq!(err.kind(), "no_previous_version");
    assert_eq!(current_version(&h, "ytdlp").as_deref(), Some("2026.10.01"));
    assert_eq!(
        h.manager.rollback(Tool::Deno).await.unwrap_err().kind(),
        "no_previous_version"
    );
}

#[tokio::test]
async fn estado_sobrevive_a_um_novo_gerenciador_sobre_a_mesma_pasta() {
    let h = harness().await;
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.01").await;
    h.manager.install(Tool::Ytdlp).await.unwrap();

    let second = ToolsManager::new(
        ToolsConfig {
            tools_dir: h.tools_dir(),
            platform: Platform::WINDOWS_X64,
            github_base_url: h.github.uri(),
            github_token: None,
            extra_env: vec![],
        },
        crate::db::Db::open_in_memory().unwrap(),
        h.settings.clone(),
        h.sink.clone(),
    )
    .unwrap();
    assert!(second.resolve(Tool::Ytdlp).unwrap().is_file());
}

// ------------------------------------------------------------------ T7

#[tokio::test]
async fn t7_update_espera_os_jobs_soltarem_o_guard() {
    let h = harness().await;
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.01").await;
    h.manager.install(Tool::Ytdlp).await.unwrap();
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.02").await;

    let guard = h.manager.acquire_run().await; // um job usando o yt-dlp
    let manager = Arc::clone(&h.manager);
    let task = tokio::spawn(async move { manager.update(Tool::Ytdlp).await });

    let started = Instant::now();
    loop {
        if phases(&h, "ytdlp").contains(&"waiting_jobs".to_string()) {
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "waiting_jobs não foi emitido"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        !task.is_finished(),
        "o update não pode concluir com o guard ativo"
    );
    assert_eq!(current_version(&h, "ytdlp").as_deref(), Some("2026.10.01"));

    drop(guard);
    let outcome = tokio::time::timeout(Duration::from_secs(20), task)
        .await
        .expect("o update deveria concluir após soltar o guard")
        .unwrap()
        .unwrap();
    assert!(matches!(outcome, InstallOutcome::Installed { .. }));
    assert_eq!(current_version(&h, "ytdlp").as_deref(), Some("2026.10.02"));
}

#[tokio::test]
async fn t7_rollback_tambem_espera_o_guard() {
    let h = harness().await;
    for tag in ["2026.10.01", "2026.10.02"] {
        publish_ytdlp(&h.github, "yt-dlp/yt-dlp", tag).await;
        h.manager.update(Tool::Ytdlp).await.unwrap();
    }
    let guard = h.manager.acquire_run().await;
    let manager = Arc::clone(&h.manager);
    let task = tokio::spawn(async move { manager.rollback(Tool::Ytdlp).await });

    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(!task.is_finished());
    assert!(phases(&h, "ytdlp").contains(&"waiting_jobs".to_string()));
    drop(guard);
    assert_eq!(task.await.unwrap().unwrap(), "2026.10.01");
}

#[tokio::test]
async fn varios_runners_seguram_leitura_ao_mesmo_tempo() {
    let h = harness().await;
    let a = h.manager.acquire_run().await;
    let b = tokio::time::timeout(Duration::from_secs(1), h.manager.acquire_run())
        .await
        .expect("leituras não se bloqueiam");
    drop((a, b));
}

// ------------------------------------------------------------------ canal, FFmpeg rolante, verificação

#[tokio::test]
async fn trocar_de_canal_instala_a_ultima_do_novo_canal() {
    let h = harness().await;
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.09.01").await;
    publish_ytdlp(
        &h.github,
        "yt-dlp/yt-dlp-nightly-builds",
        "2026.09.27.232945",
    )
    .await;
    h.manager.install(Tool::Ytdlp).await.unwrap();
    assert_eq!(current_version(&h, "ytdlp").as_deref(), Some("2026.09.01"));

    h.settings
        .update(
            serde_json::from_value::<SettingsPatch>(json!({"ytdlpChannel": "nightly"})).unwrap(),
        )
        .await
        .unwrap();
    h.manager.update(Tool::Ytdlp).await.unwrap();
    assert_eq!(
        current_version(&h, "ytdlp").as_deref(),
        Some("2026.09.27.232945")
    );
    assert_eq!(
        manifest_json(&h)["tools"]["ytdlp"]["current"]["channel"],
        "nightly"
    );

    // De volta ao estável: a versão estável é MAIS ANTIGA que a nightly, mas o canal mudou ⇒ reinstala.
    h.settings
        .update(serde_json::from_value::<SettingsPatch>(json!({"ytdlpChannel": "stable"})).unwrap())
        .await
        .unwrap();
    let outcome = h.manager.update(Tool::Ytdlp).await.unwrap();
    assert!(matches!(outcome, InstallOutcome::Installed { .. }));
    assert_eq!(current_version(&h, "ytdlp").as_deref(), Some("2026.09.01"));
    assert_eq!(
        manifest_json(&h)["tools"]["ytdlp"]["current"]["channel"],
        "stable"
    );
    assert_eq!(h.settings.get().ytdlp_channel, YtdlpChannel::Stable);
}

#[tokio::test]
async fn ffmpeg_atualiza_pela_data_do_asset_e_nao_pela_tag() {
    let h = harness().await;
    publish_ffmpeg(&h.github, "2026-09-30T19:00:56Z").await;
    h.manager.install(Tool::Ffmpeg).await.unwrap();

    // Mesmo `updated_at` ⇒ nada a fazer (a tag `latest` é sempre igual).
    let same = h.manager.check_updates(true).await.unwrap();
    assert_eq!(same.len(), 1);
    assert!(!same[0].update_available);
    assert!(matches!(
        h.manager.update(Tool::Ffmpeg).await.unwrap(),
        InstallOutcome::UpToDate { .. }
    ));

    // Auto-build novo (mesma tag, outra data).
    publish_ffmpeg(&h.github, "2026-10-01T18:57:10Z").await;
    let newer = h.manager.check_updates(true).await.unwrap();
    assert!(newer[0].update_available);
    assert_eq!(newer[0].latest, "20261001T185710Z");
    h.manager.update(Tool::Ffmpeg).await.unwrap();
    assert_eq!(
        current_version(&h, "ffmpeg").as_deref(),
        Some("20261001T185710Z")
    );
    assert_eq!(
        previous_version(&h, "ffmpeg").as_deref(),
        Some("20260930T190056Z")
    );
    assert_eq!(
        manifest_json(&h)["tools"]["ffmpeg"]["current"]["asset_updated_at"],
        "2026-10-01T18:57:10Z"
    );
}

#[tokio::test]
async fn check_updates_guarda_o_resultado_para_o_status() {
    let h = harness().await;
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.01").await;
    h.manager.install(Tool::Ytdlp).await.unwrap();
    // Só o que está instalado é verificado.
    assert_eq!(h.manager.check_updates(true).await.unwrap().len(), 1);

    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.05").await;
    let infos = h.manager.check_updates(true).await.unwrap();
    assert_eq!(infos[0].tool, Tool::Ytdlp);
    assert!(infos[0].update_available);
    assert_eq!(infos[0].latest, "2026.10.05");
    assert_eq!(infos[0].current.as_deref(), Some("2026.10.01"));

    let status = h.manager.status().await.unwrap();
    let ytdlp = status.iter().find(|s| s.tool == Tool::Ytdlp).unwrap();
    assert!(ytdlp.update_available);
    assert_eq!(ytdlp.latest_version.as_deref(), Some("2026.10.05"));
    assert!(ytdlp.last_checked.is_some());
}

#[tokio::test]
async fn check_updates_sem_rede_devolve_erro_quando_nada_foi_verificado() {
    let h = harness().await;
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.01").await;
    h.manager.install(Tool::Ytdlp).await.unwrap();
    h.github.server.reset().await; // o GitHub "saiu do ar" (404 em tudo)
    let err = h.manager.check_updates(true).await.unwrap_err();
    assert_eq!(err.kind(), "github_http");
}

// ------------------------------------------------------------------ obrigatórias e runtime JS

async fn publish_all(h: &Harness) {
    // Tudo no mesmo servidor falso: um `publish` por repositório.
    let ytdlp_assets = {
        let exe = fake_tool_bytes();
        let sums = format!("{}  yt-dlp.exe\n", sha256_hex(&exe));
        vec![
            ("yt-dlp.exe".to_string(), exe),
            ("SHA2-256SUMS".to_string(), sums.into_bytes()),
        ]
    };
    let deno_zip = make_zip(&[("deno.exe", &fake_tool_bytes())]);
    let deno_sum = format!("{}  deno-x86_64-pc-windows-msvc.zip", sha256_hex(&deno_zip));
    let ffmpeg_zip = make_zip(&[
        ("x/ffmpeg.exe", &fake_tool_bytes()),
        ("x/ffprobe.exe", &fake_tool_bytes()),
    ]);
    let ffmpeg_sum = format!(
        "{}  ffmpeg-master-latest-win64-gpl.zip",
        sha256_hex(&ffmpeg_zip)
    );
    h.github
        .publish(
            "yt-dlp/yt-dlp",
            "2026.10.01",
            "2026-10-01T00:00:00Z",
            ytdlp_assets,
        )
        .await;
    h.github
        .publish(
            "denoland/deno",
            "v2.9.8",
            "2026-10-01T00:00:00Z",
            vec![
                ("deno-x86_64-pc-windows-msvc.zip".into(), deno_zip),
                (
                    "deno-x86_64-pc-windows-msvc.zip.sha256sum".into(),
                    deno_sum.into_bytes(),
                ),
            ],
        )
        .await;
    h.github
        .publish(
            "yt-dlp/FFmpeg-Builds",
            "latest",
            "2026-09-30T19:00:56Z",
            vec![
                ("ffmpeg-master-latest-win64-gpl.zip".into(), ffmpeg_zip),
                ("checksums.sha256".into(), ffmpeg_sum.into_bytes()),
            ],
        )
        .await;
}

#[tokio::test]
async fn install_missing_instala_ytdlp_ffmpeg_e_deno_gerenciado() {
    let h = harness().await;
    publish_all(&h).await;
    let empty_path = OsString::new();

    let mut installed = h
        .manager
        .install_missing_with_path(&empty_path)
        .await
        .unwrap();
    installed.sort();
    assert_eq!(installed, [Tool::Ytdlp, Tool::Deno, Tool::Ffmpeg]);
    assert!(h.manager.resolve(Tool::Ytdlp).is_ok());
    assert!(h.manager.resolve(Tool::Ffmpeg).is_ok());
    assert!(h.manager.resolve_ffprobe().is_ok());
    // fpcalc é opcional e o bgutil só entra quando o provedor precisa.
    assert!(h.manager.resolve(Tool::Fpcalc).is_err());
    assert!(h.manager.bgutil_paths().is_none());

    // O runtime escolhido é o Deno gerenciado.
    let runtime = h.manager.resolve_js_runtime(&empty_path).await.unwrap();
    assert_eq!(runtime.kind, JsKind::Deno);
    assert_eq!(runtime.path, h.manager.resolve(Tool::Deno).unwrap());
    assert!(runtime.js_runtime_arg().starts_with("deno:"));

    // Segunda chamada: nada faltando.
    assert!(h
        .manager
        .install_missing_with_path(&empty_path)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn install_missing_nao_baixa_deno_quando_o_sistema_tem_runtime() {
    use crate::tools::testutil::copy_fake_tool;

    let h = harness().await;
    publish_all(&h).await;
    // PATH falso com um `node` 22.
    let bin = h.dir.path().join("bin do sistema");
    let node = bin.join(format!("node{}", std::env::consts::EXE_SUFFIX));
    copy_fake_tool(&node);
    let mut sidecar = node.into_os_string();
    sidecar.push(".version");
    std::fs::write(sidecar, "v22.1.0").unwrap();
    let path = std::env::join_paths([bin]).unwrap();

    let mut installed = h.manager.install_missing_with_path(&path).await.unwrap();
    installed.sort();
    assert_eq!(installed, [Tool::Ytdlp, Tool::Ffmpeg]);
    let runtime = h.manager.resolve_js_runtime(&path).await.unwrap();
    assert_eq!(runtime.kind, JsKind::Node);
    assert!(runtime.js_runtime_arg().starts_with("node:"));
}

#[tokio::test]
async fn install_missing_continua_apos_falha_de_uma_ferramenta() {
    let h = harness().await;
    // Só o yt-dlp existe no GitHub falso; FFmpeg e Deno dão 404.
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.01").await;
    let installed = h
        .manager
        .install_missing_with_path(&OsString::new())
        .await
        .unwrap();
    assert_eq!(installed, [Tool::Ytdlp]);
}

#[tokio::test]
async fn install_missing_sem_nenhuma_instalacao_devolve_o_erro() {
    let h = harness().await;
    let err = h
        .manager
        .install_missing_with_path(&OsString::new())
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "github_http");
}

#[tokio::test]
async fn deno_forcado_do_sistema_sem_deno_da_erro() {
    let h = harness().await;
    h.settings
        .update(
            serde_json::from_value::<SettingsPatch>(json!({"jsRuntime": "system-deno"})).unwrap(),
        )
        .await
        .unwrap();
    let err = h
        .manager
        .resolve_js_runtime(&OsString::new())
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "js_runtime_missing");
}

// ------------------------------------------------------------------ PO token no gerenciador

#[tokio::test]
async fn pot_args_so_com_provedor_ativo_e_bgutil_instalado() {
    let h = harness().await;
    // Auto desligado ⇒ nada.
    assert_eq!(h.manager.pot_args().await, None);

    // Autocura ligou, mas o bgutil não está instalado ⇒ segue sem token.
    h.manager.pot_policy().enable_auto().await.unwrap();
    assert_eq!(h.manager.pot_args().await, None);

    // `off` vence qualquer estado.
    h.settings
        .update(serde_json::from_value::<SettingsPatch>(json!({"potProvider": "off"})).unwrap())
        .await
        .unwrap();
    assert_eq!(h.manager.pot_args().await, None);
    h.manager.shutdown().await;
}

// ------------------------------------------------------------------ agendamento

#[test]
fn verificacao_periodica_devida() {
    let day = 24 * 3600;
    assert!(check_due(None, 1_000, day));
    assert!(!check_due(Some(1_000), 1_000 + day - 1, day));
    assert!(check_due(Some(1_000), 1_000 + day, day));
    assert!(check_due(Some(1_000), 1_000 + 8 * day, 7 * day));
    assert!(!check_due(Some(1_000), 1_000 + 6 * day, 7 * day));
    // Relógio voltou no tempo: não considera devida.
    assert!(!check_due(Some(5_000), 1_000, day));
}

#[tokio::test]
async fn startup_atualiza_o_ytdlp_quando_a_verificacao_esta_vencida() {
    let h = harness().await;
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.01").await;
    h.manager.install(Tool::Ytdlp).await.unwrap();
    // A última verificação foi há 2 dias.
    h.manager
        .db
        .kv_set(
            "tools.last_check.ytdlp",
            &(now_secs() - 2 * DAY_SECS).to_string(),
        )
        .await
        .unwrap();
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.09").await;

    Arc::clone(&h.manager).background_startup().await;
    assert_eq!(current_version(&h, "ytdlp").as_deref(), Some("2026.10.09"));
}

#[tokio::test]
async fn startup_nao_verifica_antes_do_prazo_nem_com_a_opcao_desligada() {
    let h = harness().await;
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.01").await;
    h.manager.install(Tool::Ytdlp).await.unwrap();
    // Verificada agora há pouco ⇒ não é hora.
    h.manager
        .db
        .kv_set("tools.last_check.ytdlp", &now_secs().to_string())
        .await
        .unwrap();
    publish_ytdlp(&h.github, "yt-dlp/yt-dlp", "2026.10.09").await;
    Arc::clone(&h.manager).background_startup().await;
    assert_eq!(current_version(&h, "ytdlp").as_deref(), Some("2026.10.01"));

    // Mesmo vencida, com `autoUpdateTools` desligado não atualiza.
    h.manager
        .db
        .kv_set("tools.last_check.ytdlp", "0")
        .await
        .unwrap();
    h.settings
        .update(serde_json::from_value::<SettingsPatch>(json!({"autoUpdateTools": false})).unwrap())
        .await
        .unwrap();
    Arc::clone(&h.manager).background_startup().await;
    assert_eq!(current_version(&h, "ytdlp").as_deref(), Some("2026.10.01"));
}

// ------------------------------------------------------------------ caminhos do servidor de PO token

#[test]
fn normalize_dir_devolve_o_caminho_real_sem_prefixo_verbatim() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("a").join("b");
    std::fs::create_dir_all(&nested).unwrap();
    let normalized = normalize_dir(&nested);
    assert!(!normalized.to_string_lossy().starts_with(r"\?\"));
    assert_eq!(
        std::fs::canonicalize(&normalized).unwrap(),
        std::fs::canonicalize(&nested).unwrap()
    );
    // Caminho que não existe: devolve o original em vez de falhar.
    let missing = dir.path().join("nao-existe");
    assert_eq!(normalize_dir(&missing), missing);
}

#[test]
fn servidor_do_bgutil_roda_no_node_modules_com_log() {
    let dir = tempfile::tempdir().unwrap();
    let server = dir.path().join("server");
    std::fs::create_dir_all(server.join("node_modules")).unwrap();
    let config = bgutil_server_config(PathBuf::from("deno"), &server);
    let args = (config.args)(4416);
    let args: Vec<String> = args
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        args,
        [
            "run",
            "--allow-env",
            "--allow-net",
            "--allow-ffi=.",
            "--allow-read=.",
            "../src/main.ts",
            "--port",
            "4416"
        ]
    );
    assert!(config.cwd.unwrap().ends_with("node_modules"));
    assert!(config.log_file.unwrap().ends_with("server.log"));
}

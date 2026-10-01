//! F02 — testes de rede (T10, T11, T12, T12c): ferramentas reais, GitHub e YouTube de verdade.
//! Rodam com `npm run verify:net` (`cargo test --workspace -- --ignored`). Usam diretórios
//! temporários (protocolo §7, regra 11); T11 reaproveita `REVERB_TEST_TOOLS_DIR` (padrão `.test-tools`).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use regex::Regex;
use reverb_core::tools::{run_capture, JsKind};
use reverb_core::{
    Db, MemorySink, SettingsPatch, SettingsService, Tool, ToolsConfig, ToolsManager,
};
use sysinfo::{ProcessesToUpdate, System};

const FX1: &str = "https://www.youtube.com/watch?v=jNQXAC9IVRw";
const FX2: &str = "https://music.youtube.com/watch?v=lYBUbBu4W08";

async fn manager(tools_dir: &Path) -> (Arc<ToolsManager>, Arc<SettingsService>) {
    let sink = Arc::new(MemorySink::new());
    let db = Db::open_in_memory().unwrap();
    let settings = Arc::new(
        SettingsService::new(db.clone(), sink.clone())
            .await
            .unwrap(),
    );
    let manager =
        ToolsManager::new(ToolsConfig::new(tools_dir), db, settings.clone(), sink).unwrap();
    (Arc::new(manager), settings)
}

fn patch(json: serde_json::Value) -> SettingsPatch {
    serde_json::from_value(json).unwrap()
}

fn shared_tools_dir() -> PathBuf {
    std::env::var_os("REVERB_TEST_TOOLS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tools"))
}

async fn latest_tag(repo: &str) -> String {
    let client = reqwest::Client::builder()
        .user_agent("Reverb-teste")
        .build()
        .unwrap();
    let mut request = client
        .get(format!(
            "https://api.github.com/repos/{repo}/releases/latest"
        ))
        .header("Accept", "application/vnd.github+json");
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        request = request.bearer_auth(token);
    }
    let json: serde_json::Value = request.send().await.unwrap().json().await.unwrap();
    json["tag_name"].as_str().expect("tag_name").to_string()
}

async fn run(program: &Path, args: &[&str]) -> reverb_core::tools::ToolOutput {
    run_capture(program, args, |_| {}, Duration::from_secs(180))
        .await
        .unwrap_or_else(|e| panic!("{} {args:?}: {e}", program.display()))
}

#[tokio::test]
#[ignore = "network"]
async fn t10_instala_ytdlp_deno_e_ffmpeg_reais() {
    let dir = tempfile::tempdir().unwrap();
    let (manager, _) = manager(&dir.path().join("ferramentas")).await;

    for tool in [Tool::Ytdlp, Tool::Deno, Tool::Ffmpeg] {
        manager
            .install(tool)
            .await
            .unwrap_or_else(|e| panic!("instalar {tool:?}: {e}"));
    }

    // yt-dlp --version == tag_name da API
    let ytdlp = run(&manager.resolve(Tool::Ytdlp).unwrap(), &["--version"]).await;
    assert!(ytdlp.success, "{}", ytdlp.stderr);
    assert_eq!(ytdlp.stdout.trim(), latest_tag("yt-dlp/yt-dlp").await);

    let deno = run(&manager.resolve(Tool::Deno).unwrap(), &["--version"]).await;
    assert!(
        deno.success && deno.stdout.starts_with("deno "),
        "{}",
        deno.stdout
    );

    let ffmpeg = run(&manager.resolve(Tool::Ffmpeg).unwrap(), &["-version"]).await;
    assert!(ffmpeg.success && ffmpeg.stdout.contains("ffmpeg version"));
    let ffprobe = run(&manager.resolve_ffprobe().unwrap(), &["-version"]).await;
    assert!(ffprobe.success && ffprobe.stdout.contains("ffprobe version"));

    // Instalar de novo é idempotente.
    let again = manager.update(Tool::Ytdlp).await.unwrap();
    assert!(matches!(
        again,
        reverb_core::tools::InstallOutcome::UpToDate { .. }
    ));
}

#[tokio::test]
#[ignore = "network"]
async fn t11_ytdlp_com_deno_gerenciado_reconhece_o_runtime_js() {
    let (manager, _) = manager(&shared_tools_dir()).await;
    manager.install(Tool::Ytdlp).await.unwrap();
    manager.install(Tool::Deno).await.unwrap();

    // `auto` num PATH vazio ⇒ Deno gerenciado.
    let runtime = manager
        .resolve_js_runtime(std::ffi::OsStr::new(""))
        .await
        .unwrap();
    assert_eq!(runtime.kind, JsKind::Deno);
    let arg = runtime.js_runtime_arg();

    let ytdlp = manager.resolve(Tool::Ytdlp).unwrap();
    let out = run(
        &ytdlp,
        &[
            "--ignore-config",
            "--color",
            "never",
            "-v",
            "--simulate",
            "--js-runtimes",
            &arg,
            FX1,
        ],
    )
    .await;
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    assert!(out.stderr.contains("JS runtimes: deno-"), "{}", out.stderr);
}

#[tokio::test]
#[ignore = "network"]
async fn t12_troca_de_canal_nightly_e_volta_para_o_estavel() {
    let dir = tempfile::tempdir().unwrap();
    let (manager, settings) = manager(&dir.path().join("ferramentas")).await;
    let four_parts = Regex::new(r"^\d{4}\.\d{2}\.\d{2}\.\d+$").unwrap();
    let three_parts = Regex::new(r"^\d{4}\.\d{2}\.\d{2}$").unwrap();

    settings
        .update(patch(serde_json::json!({"ytdlpChannel": "nightly"})))
        .await
        .unwrap();
    manager.install(Tool::Ytdlp).await.unwrap();
    let nightly = run(&manager.resolve(Tool::Ytdlp).unwrap(), &["--version"]).await;
    assert!(
        four_parts.is_match(nightly.stdout.trim()),
        "nightly: {}",
        nightly.stdout
    );

    settings
        .update(patch(serde_json::json!({"ytdlpChannel": "stable"})))
        .await
        .unwrap();
    manager.install(Tool::Ytdlp).await.unwrap();
    let stable = run(&manager.resolve(Tool::Ytdlp).unwrap(), &["--version"]).await;
    assert!(
        three_parts.is_match(stable.stdout.trim()),
        "estável: {}",
        stable.stdout
    );
    assert_eq!(stable.stdout.trim(), latest_tag("yt-dlp/yt-dlp").await);

    // O rollback leva de volta à nightly.
    manager.rollback(Tool::Ytdlp).await.unwrap();
    let back = run(&manager.resolve(Tool::Ytdlp).unwrap(), &["--version"]).await;
    assert!(four_parts.is_match(back.stdout.trim()));
}

fn processes_under(dir: &Path) -> Vec<String> {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    system
        .processes()
        .values()
        .filter(|p| p.exe().is_some_and(|exe| exe.starts_with(dir)))
        .map(|p| format!("{} (pid {})", p.name().to_string_lossy(), p.pid()))
        .collect()
}

#[tokio::test]
#[ignore = "network"]
async fn t12c_bgutil_real_fornece_po_token_e_nao_deixa_deno_orfao() {
    let dir = tempfile::tempdir().unwrap();
    let tools_dir = dir.path().join("ferramentas");
    let (manager, settings) = manager(&tools_dir).await;

    manager.install(Tool::Ytdlp).await.unwrap();
    manager.install(Tool::Deno).await.unwrap();
    manager
        .install(Tool::Bgutil)
        .await
        .unwrap_or_else(|e| panic!("instalar bgutil: {e}"));
    assert!(manager.bgutil_paths().is_some());

    settings
        .update(patch(serde_json::json!({"potProvider": "always"})))
        .await
        .unwrap();
    let pot_args = manager
        .pot_args()
        .await
        .expect("provedor ativo e servidor de pé");
    assert_eq!(pot_args[0], "--plugin-dirs");
    assert!(pot_args[3].starts_with("youtubepot-bgutilhttp:base_url=http://127.0.0.1:"));

    let runtime = manager
        .resolve_js_runtime(std::ffi::OsStr::new(""))
        .await
        .unwrap();
    let js_arg = runtime.js_runtime_arg();
    let mut args: Vec<&str> = vec![
        "--ignore-config",
        "--color",
        "never",
        "-v",
        "--simulate",
        "--js-runtimes",
        &js_arg,
    ];
    args.extend(pot_args.iter().map(String::as_str));
    args.extend(["--extractor-args", "youtube:player_client=mweb", FX2]);

    let out = run(&manager.resolve(Tool::Ytdlp).unwrap(), &args).await;
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    assert!(
        out.stderr.contains("Retrieved a gvs PO Token"),
        "{}",
        out.stderr
    );

    // Ao encerrar o gerenciador, o servidor (Deno) e seus filhos somem.
    assert!(
        !processes_under(&tools_dir).is_empty(),
        "o servidor deveria estar rodando"
    );
    manager.shutdown().await;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(5) && !processes_under(&tools_dir).is_empty() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(
        processes_under(&tools_dir),
        Vec::<String>::new(),
        "processos órfãos"
    );
}

//! `reverb-cli tools …` ponta a ponta, contra um GitHub falso e o `fake-tool`.

use std::path::{Path, PathBuf};
use std::sync::Once;

use serde_json::Value;
use sha2::{Digest, Sha256};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fake_tool_path() -> PathBuf {
    static BUILD: Once = Once::new();
    let exe = std::env::current_exe().unwrap();
    // target/<perfil>/deps/<teste>.exe ⇒ target/<perfil>
    let profile_dir = exe.parent().and_then(Path::parent).unwrap().to_path_buf();
    let tool = profile_dir.join(format!("fake-tool{}", std::env::consts::EXE_SUFFIX));
    BUILD.call_once(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut cmd = std::process::Command::new(env!("CARGO"));
        cmd.args(["build", "-p", "fake-tool", "--quiet"])
            .current_dir(root);
        if profile_dir.file_name().is_some_and(|n| n == "release") {
            cmd.arg("--release");
        }
        assert!(
            cmd.status().unwrap().success(),
            "falha ao compilar o fake-tool"
        );
    });
    tool
}

/// Nome do asset do yt-dlp na plataforma em que o teste roda.
fn ytdlp_asset() -> &'static str {
    if cfg!(windows) {
        "yt-dlp.exe"
    } else {
        "yt-dlp_linux"
    }
}

async fn serve_ytdlp(server: &MockServer, tag: &str) {
    let bytes = std::fs::read(fake_tool_path()).unwrap();
    let sums = format!(
        "{}  {}\n",
        hex::encode(Sha256::digest(&bytes)),
        ytdlp_asset()
    );
    let base = server.uri();
    let release = serde_json::json!({
        "tag_name": tag,
        "zipball_url": format!("{base}/zip"),
        "assets": [
            {"name": ytdlp_asset(), "browser_download_url": format!("{base}/dl/{tag}/{}", ytdlp_asset()),
             "updated_at": "2026-10-01T00:00:00Z", "size": 1},
            {"name": "SHA2-256SUMS", "browser_download_url": format!("{base}/dl/{tag}/SHA2-256SUMS"),
             "updated_at": "2026-10-01T00:00:00Z", "size": 1},
        ]
    });
    Mock::given(method("GET"))
        .and(path("/repos/yt-dlp/yt-dlp/releases/latest"))
        .respond_with(ResponseTemplate::new(200).set_body_json(release))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/dl/{tag}/{}", ytdlp_asset())))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/dl/{tag}/SHA2-256SUMS")))
        .respond_with(ResponseTemplate::new(200).set_body_string(sums))
        .mount(server)
        .await;
}

struct Cli {
    tools_dir: PathBuf,
    api: String,
}

impl Cli {
    async fn run(&self, args: &[&str]) -> std::process::Output {
        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_reverb-cli"));
        command
            .arg("--tools-dir")
            .arg(&self.tools_dir)
            .args(args)
            .env("REVERB_GITHUB_API_URL", &self.api)
            .env("FAKE_TOOL_VERSION", "2026.10.01")
            .env_remove("GITHUB_TOKEN");
        command.output().await.unwrap()
    }
}

fn stdout_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout não é JSON ({e}): {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[tokio::test]
async fn status_install_update_e_rollback() {
    let server = MockServer::start().await;
    serve_ytdlp(&server, "2026.10.01").await;
    let dir = tempfile::tempdir().unwrap();
    let cli = Cli {
        tools_dir: dir.path().join("pasta de ferramentas"),
        api: server.uri(),
    };

    // Nada instalado.
    let out = cli.run(&["tools", "status"]).await;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let status = stdout_json(&out);
    let ytdlp = status
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["tool"] == "ytdlp")
        .unwrap();
    assert_eq!(ytdlp["installed"], false);
    assert_eq!(ytdlp["required"], true);

    // Instala (e mostra o progresso no stderr).
    let out = cli.run(&["tools", "install", "ytdlp"]).await;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let json = stdout_json(&out);
    assert_eq!(json["tool"], "ytdlp");
    assert_eq!(json["outcome"]["result"], "installed");
    assert_eq!(json["outcome"]["version"], "2026.10.01");
    assert!(String::from_utf8_lossy(&out.stderr).contains("[ytdlp] downloading"));

    let status = stdout_json(&cli.run(&["tools", "status"]).await);
    let ytdlp = status
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["tool"] == "ytdlp")
        .unwrap();
    assert_eq!(ytdlp["installed"], true);
    assert_eq!(ytdlp["version"], "2026.10.01");
    assert!(Path::new(ytdlp["path"].as_str().unwrap()).is_file());

    // Instalar de novo é idempotente.
    let out = cli.run(&["tools", "install", "ytdlp"]).await;
    assert_eq!(stdout_json(&out)["outcome"]["result"], "upToDate");

    // Sem versão anterior, o rollback falha com mensagem e código ≠ 0.
    let out = cli.run(&["tools", "rollback", "ytdlp"]).await;
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("versão anterior"));

    // Atualização para a versão 2 e rollback.
    server.reset().await;
    serve_ytdlp(&server, "2026.10.02").await;
    let out = cli.run(&["tools", "update", "ytdlp"]).await;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(stdout_json(&out)["outcome"]["version"], "2026.10.02");
    let out = cli.run(&["tools", "rollback", "ytdlp"]).await;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(stdout_json(&out)["version"], "2026.10.01");
}

#[tokio::test]
async fn ferramenta_desconhecida_e_erro() {
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let cli = Cli {
        tools_dir: dir.path().join("t"),
        api: server.uri(),
    };
    let out = cli.run(&["tools", "install", "nao-existe"]).await;
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("ferramenta desconhecida"));
}

#[tokio::test]
async fn tools_dir_sozinho_nao_toca_nos_dados_do_usuario() {
    // Com --tools-dir e sem --data-dir o banco é em memória: nenhum reverb.db é criado ao lado.
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let cli = Cli {
        tools_dir: dir.path().join("t"),
        api: server.uri(),
    };
    let out = cli.run(&["tools", "status"]).await;
    assert!(out.status.success());
    assert!(!dir.path().join("reverb.db").exists());
    assert!(!dir.path().join("t").join("reverb.db").exists());
}

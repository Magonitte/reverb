//! Apoio aos testes do gerenciador de ferramentas: `fake-tool`, zips e um GitHub falso (wiremock).

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Once};

use serde_json::{json, Value};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::manager::{ToolsConfig, ToolsManager};
use super::spec::Platform;
use crate::db::Db;
use crate::events::MemorySink;
use crate::settings::SettingsService;

/// Arquivo gravado em `tests/fixtures/` (na raiz do workspace).
pub fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(relative)
}

pub fn fixture_text(relative: &str) -> String {
    std::fs::read_to_string(fixture(relative)).unwrap()
}

/// Caminho do binário `fake-tool` (compilado sob demanda, uma única vez por processo de teste).
pub fn fake_tool_path() -> PathBuf {
    static BUILD: Once = Once::new();
    let exe = std::env::current_exe().expect("current_exe");
    // target/<perfil>/deps/<teste>.exe ⇒ target/<perfil>
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("target/<perfil>");
    let tool = profile_dir.join(format!("fake-tool{}", std::env::consts::EXE_SUFFIX));
    BUILD.call_once(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut cmd = std::process::Command::new(env!("CARGO"));
        cmd.args(["build", "-p", "fake-tool", "--quiet"])
            .current_dir(root);
        if profile_dir.file_name().is_some_and(|n| n == "release") {
            cmd.arg("--release");
        }
        let status = cmd.status().expect("cargo build -p fake-tool");
        assert!(status.success(), "não foi possível compilar o fake-tool");
    });
    assert!(tool.is_file(), "fake-tool ausente em {}", tool.display());
    tool
}

/// Copia o `fake-tool` para `dest` (com bit de execução).
pub fn copy_fake_tool(dest: &Path) {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::copy(fake_tool_path(), dest).unwrap();
}

/// Bytes do `fake-tool` (para empacotar em zips).
pub fn fake_tool_bytes() -> Vec<u8> {
    std::fs::read(fake_tool_path()).unwrap()
}

/// Zip em memória com `(nome-no-zip, conteúdo)`; o bit executável é marcado nas entradas.
pub fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        for (name, bytes) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }
    buffer.into_inner()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}

#[derive(Clone)]
struct Published {
    tag: String,
    assets: Vec<(String, Vec<u8>)>,
    updated_at: String,
}

/// GitHub falso: serve `/repos/<repo>/releases/latest` e os downloads dos assets.
pub struct FakeGithub {
    pub server: MockServer,
    repos: Mutex<BTreeMap<String, Published>>,
}

impl FakeGithub {
    pub async fn start() -> Arc<Self> {
        Arc::new(Self {
            server: MockServer::start().await,
            repos: Mutex::new(BTreeMap::new()),
        })
    }

    pub fn uri(&self) -> String {
        self.server.uri()
    }

    /// Publica (ou substitui) o último release de `repo`.
    pub async fn publish(
        &self,
        repo: &str,
        tag: &str,
        updated_at: &str,
        assets: Vec<(String, Vec<u8>)>,
    ) {
        self.repos.lock().unwrap().insert(
            repo.to_string(),
            Published {
                tag: tag.to_string(),
                assets,
                updated_at: updated_at.to_string(),
            },
        );
        self.remount().await;
    }

    async fn remount(&self) {
        self.server.reset().await;
        let repos = self.repos.lock().unwrap().clone();
        for (repo, published) in repos {
            let assets: Vec<Value> = published
                .assets
                .iter()
                .map(|(name, _)| {
                    json!({
                        "name": name,
                        "browser_download_url": format!("{}/dl/{repo}/{}/{name}", self.uri(), published.tag),
                        "updated_at": published.updated_at,
                        "size": 1,
                    })
                })
                .collect();
            let body = json!({
                "tag_name": published.tag,
                "name": published.tag,
                "zipball_url": format!("{}/zipball/{repo}/{}", self.uri(), published.tag),
                "published_at": published.updated_at,
                "assets": assets,
            });
            Mock::given(method("GET"))
                .and(path(format!("/repos/{repo}/releases/latest")))
                .respond_with(ResponseTemplate::new(200).set_body_json(body))
                .mount(&self.server)
                .await;
            for (name, bytes) in &published.assets {
                Mock::given(method("GET"))
                    .and(path(format!("/dl/{repo}/{}/{name}", published.tag)))
                    .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes.clone()))
                    .mount(&self.server)
                    .await;
            }
        }
    }
}

/// Gerenciador de teste pronto: diretório temporário, banco em memória e GitHub falso.
pub struct Harness {
    pub dir: TempDir,
    pub manager: Arc<ToolsManager>,
    pub sink: Arc<MemorySink>,
    pub github: Arc<FakeGithub>,
    pub settings: Arc<SettingsService>,
}

impl Harness {
    /// `extra_env` vai para os testes de fumaça (ex.: `FAKE_TOOL_VERSION`).
    pub async fn new(extra_env: &[(&str, &str)]) -> Self {
        let github = FakeGithub::start().await;
        Self::with_github(github, extra_env).await
    }

    pub async fn with_github(github: Arc<FakeGithub>, extra_env: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let sink = Arc::new(MemorySink::new());
        let db = Db::open_in_memory().unwrap();
        let settings = Arc::new(
            SettingsService::new(db.clone(), sink.clone())
                .await
                .unwrap(),
        );
        let cfg = ToolsConfig {
            tools_dir: dir.path().join("tools"),
            // Nomes dos assets fixos nos testes; o binário falso roda em qualquer SO.
            platform: Platform::WINDOWS_X64,
            github_base_url: github.uri(),
            github_token: None,
            extra_env: extra_env
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        };
        let manager = Arc::new(ToolsManager::new(cfg, db, settings.clone(), sink.clone()).unwrap());
        Self {
            dir,
            manager,
            sink,
            github,
            settings,
        }
    }

    pub fn tools_dir(&self) -> PathBuf {
        self.dir.path().join("tools")
    }
}

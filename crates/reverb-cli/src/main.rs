mod media;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use reverb_core::paths::{default_app_data_dir, DataPaths};
use reverb_core::{
    Db, EventSink, MemorySink, SettingsPatch, SettingsService, Tool, ToolsConfig, ToolsManager,
};

#[derive(Parser)]
#[command(name = "reverb-cli", version, about = "Linha de comando do Reverb")]
struct Cli {
    /// Diretório de dados (padrão: o mesmo do app)
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,

    /// Diretório das ferramentas (padrão: <data>/tools)
    #[arg(long, global = true)]
    tools_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Imprime um JSON com informações do ambiente
    Doctor,
    /// Lê e altera as configurações
    Settings {
        #[command(subcommand)]
        action: SettingsAction,
    },
    /// Gerencia as ferramentas externas (yt-dlp, Deno, FFmpeg, fpcalc, bgutil)
    Tools {
        #[command(subcommand)]
        action: ToolsAction,
    },
    /// Analisa uma URL (vídeo, playlist, álbum ou canal)
    Analyze {
        url: String,
        /// Imprime o resultado completo em JSON
        #[arg(long)]
        json: bool,
    },
    /// Busca por texto no YouTube Music ou no YouTube
    Search {
        text: String,
        /// ytmusic | youtube
        #[arg(long, default_value = "ytmusic")]
        source: String,
        #[arg(long, default_value_t = 5)]
        limit: u32,
    },
    /// Baixa um vídeo, converte pelo perfil e move para a pasta de destino
    Download {
        url: String,
        /// original | mp3_v0 | mp3_320 | aac_256 | opus_96 | flac
        #[arg(long, default_value = "original")]
        profile: String,
        /// Pasta de destino
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum ToolsAction {
    /// Imprime o estado de cada ferramenta (JSON)
    Status,
    /// Instala: `--all` (yt-dlp, Deno, FFmpeg e fpcalc), uma ferramenta, ou (sem argumentos) só as que faltam
    Install {
        #[arg(long, conflicts_with = "tool")]
        all: bool,
        /// ytdlp | deno | ffmpeg | fpcalc | bgutil
        tool: Option<String>,
    },
    /// Atualiza uma ferramenta para a última versão (consulta o GitHub sem cache)
    Update { tool: String },
    /// Volta uma ferramenta para a versão anterior
    Rollback { tool: String },
}

#[derive(Subcommand)]
enum SettingsAction {
    /// Imprime todas as configurações (sem segredos) ou o valor de uma chave
    Get { key: Option<String> },
    /// Altera uma chave: `settings set parallelism 3` (o valor é JSON; texto simples vira string)
    Set { key: String, value: String },
}

fn resolve_paths(data_dir: Option<PathBuf>) -> Result<DataPaths> {
    if let Some(dir) = data_dir {
        return Ok(DataPaths::from_dir(dir));
    }
    let exe = std::env::current_exe().context("não foi possível localizar o executável")?;
    let exe_dir = exe.parent().context("executável sem diretório pai")?;
    let app_data =
        default_app_data_dir().context("não foi possível resolver o diretório de dados")?;
    Ok(DataPaths::resolve(exe_dir, &app_data))
}

/// Aceita JSON (`3`, `true`, `["a"]`, `"x"`); qualquer outra coisa vira string.
fn parse_value(text: &str) -> serde_json::Value {
    serde_json::from_str(text).unwrap_or_else(|_| serde_json::Value::String(text.to_string()))
}

async fn settings_service(paths: &DataPaths) -> Result<SettingsService> {
    let db = Db::open(&paths.db_file())?;
    Ok(SettingsService::new(db, Arc::new(MemorySink::new())).await?)
}

async fn run_settings(paths: &DataPaths, action: SettingsAction) -> Result<()> {
    let service = settings_service(paths).await?;
    match action {
        SettingsAction::Get { key: None } => {
            println!("{}", serde_json::to_string_pretty(&service.view())?);
        }
        SettingsAction::Get { key: Some(key) } => {
            let view = serde_json::to_value(service.view())?;
            match view.get(&key) {
                Some(value) => println!("{value}"),
                None => bail!("chave desconhecida (ou secreta): {key}"),
            }
        }
        SettingsAction::Set { key, value } => {
            let mut object = serde_json::Map::new();
            object.insert(key.clone(), parse_value(&value));
            let patch: SettingsPatch = serde_json::from_value(serde_json::Value::Object(object))
                .with_context(|| format!("não foi possível aplicar {key}={value}"))?;
            let updated = service.update(patch).await?;
            println!("{}", serde_json::to_string_pretty(&updated.view())?);
        }
    }
    Ok(())
}

/// Mostra o progresso das ferramentas no stderr (uma linha por fase / a cada 20 %).
struct StderrSink {
    last: Mutex<(String, i64)>,
}

impl EventSink for StderrSink {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        if event != "tools://progress" {
            return;
        }
        let tool = payload["tool"].as_str().unwrap_or("?");
        let phase = payload["phase"].as_str().unwrap_or("?");
        let percent = payload["percent"].as_i64().unwrap_or(0);
        let key = format!("{tool}/{phase}");
        let bucket = percent / 20;
        let mut last = self.last.lock().expect("sink");
        if last.0 != key || last.1 != bucket {
            *last = (key, bucket);
            eprintln!("[{tool}] {phase} {percent}%");
        }
    }
}

fn parse_tool(text: &str) -> Result<Tool> {
    Tool::from_id(text).with_context(|| {
        format!("ferramenta desconhecida: {text} (use ytdlp, deno, ffmpeg, fpcalc ou bgutil)")
    })
}

/// Sem `--data-dir`, o `--tools-dir` isola também o banco (em memória): testes com
/// `--tools-dir` nunca tocam nos dados reais do usuário.
async fn tools_manager(
    paths: &DataPaths,
    tools_dir: PathBuf,
    isolated: bool,
) -> Result<(ToolsManager, Arc<SettingsService>)> {
    let db = if isolated {
        Db::open_in_memory()?
    } else {
        Db::open(&paths.db_file())?
    };
    let sink: Arc<dyn EventSink> = Arc::new(StderrSink {
        last: Mutex::new((String::new(), -1)),
    });
    let settings = Arc::new(SettingsService::new(db.clone(), Arc::clone(&sink)).await?);
    let mut config = ToolsConfig::new(tools_dir);
    // Permite apontar para um GitHub falso nos testes do CLI.
    if let Ok(url) = std::env::var("REVERB_GITHUB_API_URL") {
        config.github_base_url = url;
    }
    let manager = ToolsManager::new(config, db, Arc::clone(&settings), sink)?;
    Ok((manager, settings))
}

async fn run_tools(manager: &ToolsManager, action: ToolsAction) -> Result<()> {
    match action {
        ToolsAction::Status => {
            println!(
                "{}",
                serde_json::to_string_pretty(&manager.status().await?)?
            );
        }
        ToolsAction::Install { all, tool } => {
            let targets = match (all, tool) {
                (true, _) => Tool::DEFAULT_SET.to_vec(),
                (false, Some(name)) => vec![parse_tool(&name)?],
                (false, None) => {
                    let installed = manager.install_missing().await?;
                    println!("{}", serde_json::json!({ "installed": installed }));
                    return Ok(());
                }
            };
            for tool in targets {
                let outcome = manager
                    .install(tool)
                    .await
                    .with_context(|| format!("falha ao instalar {}", tool.id()))?;
                println!(
                    "{}",
                    serde_json::json!({ "tool": tool, "outcome": outcome })
                );
            }
        }
        ToolsAction::Update { tool } => {
            let tool = parse_tool(&tool)?;
            let outcome = manager
                .update(tool)
                .await
                .with_context(|| format!("falha ao atualizar {}", tool.id()))?;
            println!(
                "{}",
                serde_json::json!({ "tool": tool, "outcome": outcome })
            );
        }
        ToolsAction::Rollback { tool } => {
            let tool = parse_tool(&tool)?;
            let version = manager
                .rollback(tool)
                .await
                .with_context(|| format!("falha ao reverter {}", tool.id()))?;
            println!(
                "{}",
                serde_json::json!({ "tool": tool, "version": version })
            );
        }
    }
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let isolated_tools = cli.tools_dir.is_some() && cli.data_dir.is_none();
    let paths = resolve_paths(cli.data_dir)?;
    let tools_dir = cli.tools_dir.unwrap_or_else(|| paths.tools_dir());

    match cli.command {
        Command::Doctor => {
            let report = serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "dataDir": paths.data_dir,
                "toolsDir": tools_dir,
                "portable": paths.portable,
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::Settings { action } => run_settings(&paths, action).await?,
        Command::Tools { action } => {
            let (manager, _) = tools_manager(&paths, tools_dir, isolated_tools).await?;
            run_tools(&manager, action).await?;
        }
        command @ (Command::Analyze { .. } | Command::Search { .. } | Command::Download { .. }) => {
            let (manager, settings) = tools_manager(&paths, tools_dir, isolated_tools).await?;
            let manager = Arc::new(manager);
            match command {
                Command::Analyze { url, json } => {
                    media::analyze(manager, settings, &url, json).await?
                }
                Command::Search {
                    text,
                    source,
                    limit,
                } => media::search(manager, settings, &text, &source, limit).await?,
                Command::Download { url, profile, out } => {
                    // Testes isolados (`--tools-dir` sem `--data-dir`) nunca tocam no tmp real.
                    let data_dir = if isolated_tools {
                        std::env::temp_dir().join("reverb-cli")
                    } else {
                        paths.data_dir.clone()
                    };
                    media::download(manager, settings, data_dir, &url, &profile, &out).await?
                }
                _ => unreachable!("filtrado acima"),
            }
        }
    }
    Ok(())
}

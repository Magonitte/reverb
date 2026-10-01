use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use reverb_core::paths::{default_app_data_dir, DataPaths};
use reverb_core::{Db, MemorySink, SettingsPatch, SettingsService};

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();
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
    }
    Ok(())
}

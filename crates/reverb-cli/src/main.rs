use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use reverb_core::paths::{default_app_data_dir, DataPaths};

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

fn main() -> Result<()> {
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
    }
    Ok(())
}

//! Flags headless do executável (arquitetura §17): rodam antes do `tauri::Builder`,
//! sem criar janela, para testes automatizados.

use std::path::{Path, PathBuf};

use reverb_core::paths::{default_app_data_dir, DataPaths};

/// Resolve os diretórios de dados: `REVERB_DATA_DIR` só em debug; senão modo portátil ou padrão.
pub fn resolve_data_paths() -> Result<DataPaths, String> {
    if cfg!(debug_assertions) {
        if let Some(dir) = std::env::var_os("REVERB_DATA_DIR") {
            return Ok(DataPaths::from_dir(PathBuf::from(dir)));
        }
    }
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let exe_dir = exe.parent().ok_or("executável sem diretório pai")?;
    let app_data = default_app_data_dir().ok_or("sem diretório de dados do usuário")?;
    Ok(DataPaths::resolve(exe_dir, &app_data))
}

/// Pasta das ferramentas: `REVERB_TOOLS_DIR` só em debug; senão `<dados>/tools` (arquitetura §4).
pub fn resolve_tools_dir(paths: &DataPaths) -> PathBuf {
    if cfg!(debug_assertions) {
        if let Some(dir) = std::env::var_os("REVERB_TOOLS_DIR") {
            return PathBuf::from(dir);
        }
    }
    paths.tools_dir()
}

/// Modos que precisam do plugin de atualização e por isso rodam dentro do `setup()`.
pub enum UpdateMode {
    Check(PathBuf),
    Install,
}

/// Autoteste sai já (não precisa do Tauri). Atualização devolve o modo para o `setup()`.
/// `Err(código)` encerra o processo.
pub fn classify(args: &[String]) -> Result<Option<UpdateMode>, i32> {
    if let Some(position) = args.iter().position(|a| a == "--headless-selftest") {
        let Some(output) = args.get(position + 1) else {
            eprintln!("--headless-selftest exige o caminho do arquivo de saída");
            return Err(2);
        };
        return Err(match selftest(Path::new(output)) {
            Ok(()) => 0,
            Err(message) => {
                eprintln!("autoteste falhou: {message}");
                1
            }
        });
    }
    if let Some(position) = args.iter().position(|a| a == "--headless-update-check") {
        let Some(output) = args.get(position + 1).filter(|p| !p.starts_with("--")) else {
            eprintln!("--headless-update-check exige o caminho do arquivo de saída");
            return Err(2);
        };
        return Ok(Some(UpdateMode::Check(PathBuf::from(output))));
    }
    if args.iter().any(|a| a == "--headless-update-install") {
        return Ok(Some(UpdateMode::Install));
    }
    Ok(None)
}

fn selftest(output: &Path) -> Result<(), String> {
    let paths = resolve_data_paths()?;
    let report = serde_json::json!({
        "ok": true,
        "version": env!("CARGO_PKG_VERSION"),
        "dataDir": paths.data_dir,
        "portable": paths.portable,
    });
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(output, report.to_string()).map_err(|e| e.to_string())
}

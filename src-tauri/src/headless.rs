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

/// Trata as flags headless. Devolve `Some(código)` se o processo deve sair já.
pub fn handle(args: &[String]) -> Option<i32> {
    let position = args.iter().position(|a| a == "--headless-selftest")?;
    let Some(output) = args.get(position + 1) else {
        eprintln!("--headless-selftest exige o caminho do arquivo de saída");
        return Some(2);
    };
    Some(match selftest(Path::new(output)) {
        Ok(()) => 0,
        Err(message) => {
            eprintln!("autoteste falhou: {message}");
            1
        }
    })
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

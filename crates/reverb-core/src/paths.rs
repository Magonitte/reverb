//! Resolução dos diretórios de dados (arquitetura §4). Recebe os caminhos como parâmetros
//! para ser testável sem tocar nas pastas reais do usuário.

use std::path::{Path, PathBuf};

use crate::settings::Settings;

pub const APP_IDENTIFIER: &str = "com.reverb.desktop";
pub const PORTABLE_MARKER: &str = "portable.txt";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataPaths {
    pub data_dir: PathBuf,
    pub portable: bool,
}

impl DataPaths {
    /// Modo portátil se existir `portable.txt` ao lado do executável (`<exe_dir>/data`);
    /// senão usa `app_data_dir`.
    pub fn resolve(exe_dir: &Path, app_data_dir: &Path) -> Self {
        if exe_dir.join(PORTABLE_MARKER).is_file() {
            Self {
                data_dir: exe_dir.join("data"),
                portable: true,
            }
        } else {
            Self::from_dir(app_data_dir)
        }
    }

    /// Usa um diretório explícito (`--data-dir`, `REVERB_DATA_DIR`), ignorando o modo portátil.
    pub fn from_dir(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            portable: false,
        }
    }

    pub fn db_file(&self) -> PathBuf {
        self.data_dir.join("reverb.db")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    pub fn tools_dir(&self) -> PathBuf {
        self.data_dir.join("tools")
    }

    pub fn tmp_dir(&self) -> PathBuf {
        self.data_dir.join("tmp")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.data_dir.join("cache")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.data_dir.join("backups")
    }
}

/// Mesmo caminho que o `app_data_dir` do Tauri usa no Windows e no Linux.
pub fn default_app_data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join(APP_IDENTIFIER))
}

/// Pasta de música padrão: `dirs::audio_dir()/Reverb` (fallback `~/Music/Reverb`).
pub fn default_music_dir() -> PathBuf {
    dirs::audio_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Music")))
        .unwrap_or_else(|| PathBuf::from("Music"))
        .join("Reverb")
}

/// Pasta de destino efetiva: `outputDir` das configurações ou a pasta de música padrão.
pub fn resolve_output_dir(settings: &Settings) -> PathBuf {
    if settings.output_dir.is_empty() {
        default_music_dir()
    } else {
        PathBuf::from(&settings.output_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn modo_normal_usa_app_data_dir() {
        let exe = tempdir().unwrap();
        let app_data = tempdir().unwrap();
        let paths = DataPaths::resolve(exe.path(), app_data.path());
        assert!(!paths.portable);
        assert_eq!(paths.data_dir, app_data.path());
    }

    #[test]
    fn modo_portatil_usa_data_ao_lado_do_exe() {
        let exe = tempdir().unwrap();
        std::fs::write(exe.path().join(PORTABLE_MARKER), "").unwrap();
        let app_data = tempdir().unwrap();
        let paths = DataPaths::resolve(exe.path(), app_data.path());
        assert!(paths.portable);
        assert_eq!(paths.data_dir, exe.path().join("data"));
    }

    #[test]
    fn subpastas_seguem_o_layout() {
        let paths = DataPaths::from_dir("/x");
        assert_eq!(paths.db_file(), Path::new("/x").join("reverb.db"));
        assert_eq!(paths.logs_dir(), Path::new("/x").join("logs"));
        assert_eq!(paths.tools_dir(), Path::new("/x").join("tools"));
        assert_eq!(paths.tmp_dir(), Path::new("/x").join("tmp"));
        assert_eq!(paths.cache_dir(), Path::new("/x").join("cache"));
        assert_eq!(paths.backups_dir(), Path::new("/x").join("backups"));
    }

    #[test]
    fn pasta_de_saida_usa_a_configuracao_ou_o_padrao() {
        let mut settings = Settings::default();
        assert!(resolve_output_dir(&settings).ends_with("Reverb"));
        settings.output_dir = "/musicas".to_string();
        assert_eq!(resolve_output_dir(&settings), PathBuf::from("/musicas"));
    }

    #[test]
    fn diretorio_padrao_termina_com_o_identificador() {
        if let Some(dir) = default_app_data_dir() {
            assert!(dir.ends_with(APP_IDENTIFIER));
        }
    }
}

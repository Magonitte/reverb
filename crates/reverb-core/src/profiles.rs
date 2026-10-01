//! Perfis de saída (arquitetura §7). O yt-dlp sempre baixa o original; o perfil decide se o
//! nosso passo de ffmpeg converte.

use serde::Serialize;
use ts_rs::TS;

/// Extensões que o perfil `original` mantém sem recodificar.
pub const KEEPABLE_EXTENSIONS: &[&str] = &["opus", "m4a", "mp3", "ogg", "flac", "wav"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub id: &'static str,
    pub i18n_key: &'static str,
    /// Vazio no `original`: a extensão é a da fonte.
    pub ext: &'static str,
    /// Argumentos de codec do ffmpeg (vazio = não converter).
    pub args: &'static [&'static str],
    pub reencodes: bool,
}

pub const PROFILES: &[Profile] = &[
    Profile {
        id: "original",
        i18n_key: "profiles.original",
        ext: "",
        args: &[],
        reencodes: false,
    },
    Profile {
        id: "mp3_v0",
        i18n_key: "profiles.mp3V0",
        ext: "mp3",
        args: &["-c:a", "libmp3lame", "-q:a", "0"],
        reencodes: true,
    },
    Profile {
        id: "mp3_320",
        i18n_key: "profiles.mp3320",
        ext: "mp3",
        args: &["-c:a", "libmp3lame", "-b:a", "320k"],
        reencodes: true,
    },
    Profile {
        id: "aac_256",
        i18n_key: "profiles.aac256",
        ext: "m4a",
        args: &["-c:a", "aac", "-b:a", "256k"],
        reencodes: true,
    },
    Profile {
        id: "opus_96",
        i18n_key: "profiles.opus96",
        ext: "opus",
        args: &["-c:a", "libopus", "-b:a", "96k"],
        reencodes: true,
    },
    Profile {
        id: "flac",
        i18n_key: "profiles.flac",
        ext: "flac",
        args: &["-c:a", "flac", "-compression_level", "8"],
        reencodes: true,
    },
];

/// Fonte exótica no perfil `original`: recodifica para Opus 160k (§7).
const FALLBACK_ARGS: &[&str] = &["-c:a", "libopus", "-b:a", "160k"];

/// Versão serializável para a UI.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProfileInfo {
    pub id: String,
    pub i18n_key: String,
    pub ext: String,
    pub reencodes: bool,
}

/// Perfil aplicado a uma fonte concreta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProfile {
    pub id: &'static str,
    pub ext: String,
    pub args: Vec<String>,
    pub reencodes: bool,
    /// `original` com fonte fora da lista: foi trocado por Opus 160k (registrar aviso no log).
    pub fallback: bool,
}

impl ResolvedProfile {
    /// Há conversão a fazer (o arquivo baixado não serve como está).
    pub fn needs_conversion(&self) -> bool {
        !self.args.is_empty()
    }
}

impl Profile {
    pub fn info(&self) -> ProfileInfo {
        ProfileInfo {
            id: self.id.to_string(),
            i18n_key: self.i18n_key.to_string(),
            ext: self.ext.to_string(),
            reencodes: self.reencodes,
        }
    }

    /// Aplica o perfil à extensão do arquivo baixado (`original_ext` sem ponto).
    pub fn resolve(&self, original_ext: &str) -> ResolvedProfile {
        if self.id != "original" {
            return ResolvedProfile {
                id: self.id,
                ext: self.ext.to_string(),
                args: self.args.iter().map(|a| a.to_string()).collect(),
                reencodes: self.reencodes,
                fallback: false,
            };
        }
        let ext = original_ext.to_ascii_lowercase();
        if KEEPABLE_EXTENSIONS.contains(&ext.as_str()) {
            ResolvedProfile {
                id: self.id,
                ext,
                args: Vec::new(),
                reencodes: false,
                fallback: false,
            }
        } else {
            ResolvedProfile {
                id: self.id,
                ext: "opus".to_string(),
                args: FALLBACK_ARGS.iter().map(|a| a.to_string()).collect(),
                reencodes: true,
                fallback: true,
            }
        }
    }
}

pub fn profile(id: &str) -> Option<&'static Profile> {
    PROFILES.iter().find(|p| p.id == id)
}

/// Perfil `original` aplicado a uma fonte.
pub fn profile_for_source(original_ext: &str) -> ResolvedProfile {
    PROFILES[0].resolve(original_ext)
}

#[cfg(test)]
mod tests;

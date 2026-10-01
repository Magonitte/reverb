//! Descrição declarativa das ferramentas externas (arquitetura §16).

use regex::Regex;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::settings::YtdlpChannel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Tool {
    Ytdlp,
    Deno,
    Ffmpeg,
    Fpcalc,
    Bgutil,
}

impl Tool {
    pub const ALL: [Tool; 5] = [
        Tool::Ytdlp,
        Tool::Deno,
        Tool::Ffmpeg,
        Tool::Fpcalc,
        Tool::Bgutil,
    ];

    /// Instaladas por `--all` (o bgutil só entra quando o provedor de PO token precisa).
    pub const DEFAULT_SET: [Tool; 4] = [Tool::Ytdlp, Tool::Deno, Tool::Ffmpeg, Tool::Fpcalc];

    /// Identificador usado em pastas, manifesto, eventos e no CLI.
    pub fn id(self) -> &'static str {
        match self {
            Tool::Ytdlp => "ytdlp",
            Tool::Deno => "deno",
            Tool::Ffmpeg => "ffmpeg",
            Tool::Fpcalc => "fpcalc",
            Tool::Bgutil => "bgutil",
        }
    }

    pub fn from_id(text: &str) -> Option<Tool> {
        match text.to_ascii_lowercase().as_str() {
            "ytdlp" | "yt-dlp" => Some(Tool::Ytdlp),
            "deno" => Some(Tool::Deno),
            "ffmpeg" => Some(Tool::Ffmpeg),
            "fpcalc" => Some(Tool::Fpcalc),
            "bgutil" => Some(Tool::Bgutil),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Aarch64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

impl Platform {
    pub const WINDOWS_X64: Platform = Platform {
        os: Os::Windows,
        arch: Arch::X86_64,
    };
    pub const LINUX_X64: Platform = Platform {
        os: Os::Linux,
        arch: Arch::X86_64,
    };

    pub fn current() -> Platform {
        Platform {
            os: if cfg!(windows) {
                Os::Windows
            } else {
                Os::Linux
            },
            arch: if cfg!(target_arch = "aarch64") {
                Arch::Aarch64
            } else {
                Arch::X86_64
            },
        }
    }

    /// Sufixo dos executáveis (`.exe` no Windows).
    pub fn exe_suffix(self) -> &'static str {
        match self.os {
            Os::Windows => ".exe",
            Os::Linux => "",
        }
    }

    pub fn exe_name(self, base: &str) -> String {
        format!("{base}{}", self.exe_suffix())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageKind {
    /// Executável solto (yt-dlp).
    Exe,
    Zip,
    TarXz,
    TarGz,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumSource {
    /// Um arquivo do release com linhas `<hash>  <nome>`.
    SumsFile(&'static str),
    /// Um arquivo `<asset><sufixo>` com o hash (1º token de 64 hex).
    PerAsset(&'static str),
    /// Sem checksum publicado: registra aviso.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionKind {
    /// `YYYY.MM.DD[.HHMMSS]`
    Date,
    Semver,
    /// Tag rolante: compara o `updated_at` do asset.
    Rolling,
}

#[derive(Debug, Clone)]
pub struct BinarySpec {
    /// Nome do arquivo a localizar dentro do pacote extraído.
    pub find: String,
    /// Nome final dentro de `tools/<tool>/<versão>/`.
    pub install_as: String,
}

#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub tool: Tool,
    pub repo: &'static str,
    pub asset_regex: String,
    pub package: PackageKind,
    /// O primeiro é o binário principal (`resolve`).
    pub binaries: Vec<BinarySpec>,
    pub checksum: ChecksumSource,
    pub version_args: &'static [&'static str],
    pub version_regex: &'static str,
    pub version_kind: VersionKind,
}

impl ToolSpec {
    pub fn asset_matcher(&self) -> Regex {
        Regex::new(&self.asset_regex).expect("regex do asset")
    }

    pub fn main_binary(&self) -> Option<&BinarySpec> {
        self.binaries.first()
    }
}

fn bin(find: String, install_as: String) -> BinarySpec {
    BinarySpec { find, install_as }
}

/// Especificação da ferramenta para a plataforma, ou `None` se não houver build suportado.
pub fn spec_for(tool: Tool, channel: YtdlpChannel, platform: Platform) -> Option<ToolSpec> {
    if platform.arch != Arch::X86_64 {
        return None;
    }
    let win = platform.os == Os::Windows;
    let spec = match tool {
        Tool::Ytdlp => ToolSpec {
            tool,
            repo: match channel {
                YtdlpChannel::Stable => "yt-dlp/yt-dlp",
                YtdlpChannel::Nightly => "yt-dlp/yt-dlp-nightly-builds",
            },
            asset_regex: if win {
                r"^yt-dlp\.exe$".into()
            } else {
                r"^yt-dlp_linux$".into()
            },
            package: PackageKind::Exe,
            binaries: vec![bin(
                if win { "yt-dlp.exe" } else { "yt-dlp_linux" }.into(),
                platform.exe_name("yt-dlp"),
            )],
            checksum: ChecksumSource::SumsFile("SHA2-256SUMS"),
            version_args: &["--version"],
            version_regex: r"(?m)^\s*(\d{4}\.\d{2}\.\d{2}(?:\.\d+)?)\s*$",
            version_kind: VersionKind::Date,
        },
        Tool::Deno => ToolSpec {
            tool,
            repo: "denoland/deno",
            asset_regex: if win {
                r"^deno-x86_64-pc-windows-msvc\.zip$".into()
            } else {
                r"^deno-x86_64-unknown-linux-gnu\.zip$".into()
            },
            package: PackageKind::Zip,
            binaries: vec![bin(platform.exe_name("deno"), platform.exe_name("deno"))],
            checksum: ChecksumSource::PerAsset(".sha256sum"),
            version_args: &["--version"],
            version_regex: r"(?m)^deno (\d+\.\d+\.\d+\S*)",
            version_kind: VersionKind::Semver,
        },
        Tool::Ffmpeg => ToolSpec {
            tool,
            repo: "yt-dlp/FFmpeg-Builds",
            asset_regex: if win {
                r"^ffmpeg-master-latest-win64-gpl\.zip$".into()
            } else {
                r"^ffmpeg-master-latest-linux64-gpl\.tar\.xz$".into()
            },
            package: if win {
                PackageKind::Zip
            } else {
                PackageKind::TarXz
            },
            binaries: vec![
                bin(platform.exe_name("ffmpeg"), platform.exe_name("ffmpeg")),
                bin(platform.exe_name("ffprobe"), platform.exe_name("ffprobe")),
            ],
            checksum: ChecksumSource::SumsFile("checksums.sha256"),
            version_args: &["-version"],
            version_regex: r"(?m)^ffmpeg version (\S+)",
            version_kind: VersionKind::Rolling,
        },
        Tool::Fpcalc => ToolSpec {
            tool,
            repo: "acoustid/chromaprint",
            asset_regex: if win {
                r"^chromaprint-fpcalc-[\d.]+-windows-x86_64\.zip$".into()
            } else {
                r"^chromaprint-fpcalc-[\d.]+-linux-x86_64\.tar\.gz$".into()
            },
            package: if win {
                PackageKind::Zip
            } else {
                PackageKind::TarGz
            },
            binaries: vec![bin(
                platform.exe_name("fpcalc"),
                platform.exe_name("fpcalc"),
            )],
            checksum: ChecksumSource::None,
            version_args: &["-version"],
            version_regex: r"(?im)version (\d+\.\d+\.\d+)",
            version_kind: VersionKind::Semver,
        },
        Tool::Bgutil => ToolSpec {
            tool,
            repo: "Brainicism/bgutil-ytdlp-pot-provider",
            asset_regex: r"^bgutil-ytdlp-pot-provider\.zip$".into(),
            package: PackageKind::Zip,
            binaries: Vec::new(),
            checksum: ChecksumSource::None,
            version_args: &[],
            version_regex: r"(\d+\.\d+\.\d+)",
            version_kind: VersionKind::Semver,
        },
    };
    Some(spec)
}

#[cfg(test)]
mod tests;

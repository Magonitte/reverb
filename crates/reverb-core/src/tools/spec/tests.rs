//! T1: seleção de asset por (SO, arquitetura) com as respostas reais gravadas do GitHub.

use super::*;
use crate::tools::github::Release;
use crate::tools::testutil::fixture_text;

fn release(name: &str) -> Release {
    serde_json::from_str(&fixture_text(&format!("github/{name}.release.json"))).unwrap()
}

/// O nome do único asset que combina com a regex da ferramenta.
fn picked(tool: Tool, channel: YtdlpChannel, platform: Platform, fixture: &str) -> String {
    let spec = spec_for(tool, channel, platform).expect("plataforma suportada");
    let release = release(fixture);
    let matcher = spec.asset_matcher();
    let matches: Vec<_> = release
        .assets
        .iter()
        .filter(|a| matcher.is_match(&a.name))
        .map(|a| a.name.clone())
        .collect();
    assert_eq!(matches.len(), 1, "{tool:?}/{platform:?}: {matches:?}");
    matches.into_iter().next().unwrap()
}

#[test]
fn ytdlp_estavel_e_nightly() {
    for (channel, fixture) in [
        (YtdlpChannel::Stable, "ytdlp-stable"),
        (YtdlpChannel::Nightly, "ytdlp-nightly"),
    ] {
        assert_eq!(
            picked(Tool::Ytdlp, channel, Platform::WINDOWS_X64, fixture),
            "yt-dlp.exe"
        );
        assert_eq!(
            picked(Tool::Ytdlp, channel, Platform::LINUX_X64, fixture),
            "yt-dlp_linux"
        );
    }
    let stable = spec_for(Tool::Ytdlp, YtdlpChannel::Stable, Platform::WINDOWS_X64).unwrap();
    let nightly = spec_for(Tool::Ytdlp, YtdlpChannel::Nightly, Platform::WINDOWS_X64).unwrap();
    assert_eq!(stable.repo, "yt-dlp/yt-dlp");
    assert_eq!(nightly.repo, "yt-dlp/yt-dlp-nightly-builds");
}

#[test]
fn deno_windows_e_linux() {
    assert_eq!(
        picked(
            Tool::Deno,
            YtdlpChannel::Stable,
            Platform::WINDOWS_X64,
            "deno"
        ),
        "deno-x86_64-pc-windows-msvc.zip"
    );
    assert_eq!(
        picked(
            Tool::Deno,
            YtdlpChannel::Stable,
            Platform::LINUX_X64,
            "deno"
        ),
        "deno-x86_64-unknown-linux-gnu.zip"
    );
}

#[test]
fn ffmpeg_ignora_as_variantes_shared() {
    assert_eq!(
        picked(
            Tool::Ffmpeg,
            YtdlpChannel::Stable,
            Platform::WINDOWS_X64,
            "ffmpeg"
        ),
        "ffmpeg-master-latest-win64-gpl.zip"
    );
    assert_eq!(
        picked(
            Tool::Ffmpeg,
            YtdlpChannel::Stable,
            Platform::LINUX_X64,
            "ffmpeg"
        ),
        "ffmpeg-master-latest-linux64-gpl.tar.xz"
    );
}

#[test]
fn fpcalc_windows_e_linux() {
    assert_eq!(
        picked(
            Tool::Fpcalc,
            YtdlpChannel::Stable,
            Platform::WINDOWS_X64,
            "fpcalc"
        ),
        "chromaprint-fpcalc-1.6.1-windows-x86_64.zip"
    );
    assert_eq!(
        picked(
            Tool::Fpcalc,
            YtdlpChannel::Stable,
            Platform::LINUX_X64,
            "fpcalc"
        ),
        "chromaprint-fpcalc-1.6.1-linux-x86_64.tar.gz"
    );
}

#[test]
fn bgutil_usa_o_zip_do_plugin() {
    assert_eq!(
        picked(
            Tool::Bgutil,
            YtdlpChannel::Stable,
            Platform::WINDOWS_X64,
            "bgutil"
        ),
        "bgutil-ytdlp-pot-provider.zip"
    );
    assert!(release("bgutil").zipball_url.is_some());
}

#[test]
fn arquivos_de_checksum_existem_nos_releases_reais() {
    let ytdlp = release("ytdlp-stable");
    let nightly = release("ytdlp-nightly");
    let deno = release("deno");
    let ffmpeg = release("ffmpeg");
    for (tool, release) in [
        (Tool::Ytdlp, &ytdlp),
        (Tool::Deno, &deno),
        (Tool::Ffmpeg, &ffmpeg),
    ] {
        let spec = spec_for(tool, YtdlpChannel::Stable, Platform::WINDOWS_X64).unwrap();
        let asset = release
            .assets
            .iter()
            .find(|a| spec.asset_matcher().is_match(&a.name))
            .unwrap();
        let expected = match spec.checksum {
            ChecksumSource::SumsFile(name) => name.to_string(),
            ChecksumSource::PerAsset(suffix) => format!("{}{suffix}", asset.name),
            ChecksumSource::None => panic!("{tool:?} deveria ter checksum"),
        };
        assert!(
            release.asset(&expected).is_some(),
            "{tool:?}: falta {expected}"
        );
    }
    assert!(nightly.asset("SHA2-256SUMS").is_some());
}

#[test]
fn fpcalc_e_bgutil_nao_tem_checksum() {
    for tool in [Tool::Fpcalc, Tool::Bgutil] {
        let spec = spec_for(tool, YtdlpChannel::Stable, Platform::WINDOWS_X64).unwrap();
        assert_eq!(spec.checksum, ChecksumSource::None);
    }
}

#[test]
fn arquitetura_nao_suportada_devolve_none() {
    let arm = Platform {
        os: Os::Linux,
        arch: Arch::Aarch64,
    };
    for tool in Tool::ALL {
        assert!(spec_for(tool, YtdlpChannel::Stable, arm).is_none());
    }
}

#[test]
fn nomes_dos_binarios_seguem_a_plataforma() {
    let win = spec_for(Tool::Ffmpeg, YtdlpChannel::Stable, Platform::WINDOWS_X64).unwrap();
    let names: Vec<_> = win.binaries.iter().map(|b| b.install_as.as_str()).collect();
    assert_eq!(names, ["ffmpeg.exe", "ffprobe.exe"]);
    let linux = spec_for(Tool::Ffmpeg, YtdlpChannel::Stable, Platform::LINUX_X64).unwrap();
    let names: Vec<_> = linux
        .binaries
        .iter()
        .map(|b| b.install_as.as_str())
        .collect();
    assert_eq!(names, ["ffmpeg", "ffprobe"]);
}

#[test]
fn ids_das_ferramentas_ida_e_volta() {
    for tool in Tool::ALL {
        assert_eq!(Tool::from_id(tool.id()), Some(tool));
    }
    assert_eq!(Tool::from_id("yt-dlp"), Some(Tool::Ytdlp));
    assert_eq!(Tool::from_id("nada"), None);
}

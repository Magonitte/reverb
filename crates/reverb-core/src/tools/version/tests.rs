//! T3: parse e comparação de versões.

use std::cmp::Ordering;

use super::*;
use crate::settings::YtdlpChannel;
use crate::tools::spec::{spec_for, Platform};

fn spec(tool: Tool) -> ToolSpec {
    spec_for(tool, YtdlpChannel::Stable, Platform::WINDOWS_X64).unwrap()
}

#[test]
fn ytdlp_compara_por_data() {
    let kind = VersionKind::Date;
    assert_eq!(
        compare(kind, "2026.08.19", "2026.09.27.232945"),
        Ordering::Less
    );
    assert_eq!(
        compare(kind, "2026.09.27.232945", "2026.08.19"),
        Ordering::Greater
    );
    assert_eq!(
        compare(kind, "2026.09.27", "2026.09.27.232945"),
        Ordering::Less
    );
    assert_eq!(
        compare(kind, "2026.09.27.010101", "2026.09.27.232945"),
        Ordering::Less
    );
    assert_eq!(compare(kind, "2026.08.19", "2026.08.19"), Ordering::Equal);
    // Não é comparação de texto: 2026.9.1 não vem depois de 2026.10.1 por causa dos zeros.
    assert_eq!(compare(kind, "2026.09.30", "2026.10.01"), Ordering::Less);
}

#[test]
fn deno_tag_e_saida_do_binario_sao_iguais() {
    let spec = spec(Tool::Deno);
    let output = "deno 2.9.7 (stable, release, x86_64-pc-windows-msvc)\nv8 14.5.201.2-rusty\ntypescript 5.9.2\n";
    let reported = parse_version(&spec, output).unwrap();
    let from_tag = tag_to_version(Tool::Deno, "v2.9.7");
    assert_eq!(reported, "2.9.7");
    assert_eq!(from_tag, "2.9.7");
    assert_eq!(
        compare(VersionKind::Semver, &reported, &from_tag),
        Ordering::Equal
    );
    assert_eq!(
        compare(VersionKind::Semver, "2.9.7", "2.10.0"),
        Ordering::Less
    );
    assert_eq!(
        compare(VersionKind::Semver, "v2.9.7", "2.9.7"),
        Ordering::Equal
    );
}

#[test]
fn ytdlp_le_a_versao_da_saida() {
    let spec = spec(Tool::Ytdlp);
    assert_eq!(parse_version(&spec, "2026.08.19\n").unwrap(), "2026.08.19");
    assert_eq!(
        parse_version(&spec, "2026.09.27.232945\r\n").unwrap(),
        "2026.09.27.232945"
    );
    assert_eq!(parse_version(&spec, "erro qualquer"), None);
    assert_eq!(tag_to_version(Tool::Ytdlp, "2026.08.19"), "2026.08.19");
}

#[test]
fn ffmpeg_e_fpcalc_leem_a_versao() {
    let ffmpeg = spec(Tool::Ffmpeg);
    let out = "ffmpeg version N-121987-gabcdef123-20260930 Copyright (c) 2000-2026 the FFmpeg developers\nbuilt with gcc 15";
    assert_eq!(
        parse_version(&ffmpeg, out).unwrap(),
        "N-121987-gabcdef123-20260930"
    );
    let fpcalc = spec(Tool::Fpcalc);
    assert_eq!(
        parse_version(&fpcalc, "fpcalc version 1.6.1\n").unwrap(),
        "1.6.1"
    );
    assert_eq!(tag_to_version(Tool::Fpcalc, "v1.6.1"), "1.6.1");
}

#[test]
fn ffmpeg_compara_por_data_de_atualizacao() {
    let kind = VersionKind::Rolling;
    assert_eq!(
        compare(kind, "2026-09-30T19:00:51Z", "2026-09-29T19:00:51Z"),
        Ordering::Greater
    );
    assert_eq!(
        compare(kind, "2026-09-30T19:00:51Z", "2026-09-30T19:00:51Z"),
        Ordering::Equal
    );
}

#[test]
fn semver_invalido_cai_para_comparacao_numerica() {
    assert_eq!(compare(VersionKind::Semver, "1.6", "1.6.1"), Ordering::Less);
    assert_eq!(
        compare(VersionKind::Semver, "2.0.0-rc.1", "2.0.0"),
        Ordering::Less
    );
}

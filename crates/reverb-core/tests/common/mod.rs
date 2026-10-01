//! Apoio dos testes de integração que usam as ferramentas reais de `.test-tools/`
//! (instaladas por `npm run test:prepare`). Se faltarem, o teste **falha** com instrução clara.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn tools_root() -> PathBuf {
    std::env::var_os("REVERB_TEST_TOOLS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tools"))
}

/// Pasta da versão atual de uma ferramenta (`ytdlp`, `ffmpeg`, `deno`…).
pub fn tool_dir(id: &str) -> PathBuf {
    let root = tools_root();
    let manifest = std::fs::read_to_string(root.join("manifest.json")).unwrap_or_else(|_| {
        panic!(
            "ferramentas de teste ausentes em {}: rode `npm run test:prepare`",
            root.display()
        )
    });
    let json: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    let version = json["tools"][id]["current"]["version"]
        .as_str()
        .unwrap_or_else(|| {
            panic!("{id} não instalado em .test-tools: rode `npm run test:prepare`")
        });
    root.join(id).join(version)
}

pub fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

pub fn ffmpeg() -> PathBuf {
    tool_dir("ffmpeg").join(exe("ffmpeg"))
}

pub fn ffprobe() -> PathBuf {
    tool_dir("ffmpeg").join(exe("ffprobe"))
}

/// 10 s de ruído rosa **estéreo de verdade** em Opus 160k. Ruído (não seno) para o VBR produzir
/// bitrate realista; dois geradores independentes (`seed` diferente) porque `-ac 2` sobre uma
/// fonte mono duplicaria o canal e o joint stereo derrubaria o V0 para ~150 kbps.
pub async fn pink_noise_opus(dir: &Path) -> PathBuf {
    let out = dir.join("ruido.opus");
    let mut args: Vec<String> = [
        "-hide_banner",
        "-nostdin",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "anoisesrc=color=pink:amplitude=0.3:duration=10:seed=1",
        "-f",
        "lavfi",
        "-i",
        "anoisesrc=color=pink:amplitude=0.3:duration=10:seed=2",
        "-filter_complex",
        "[0:a][1:a]amerge=inputs=2",
        "-c:a",
        "libopus",
        "-b:a",
        "160k",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    args.push(out.to_string_lossy().into_owned());
    let result =
        reverb_core::tools::run_capture(ffmpeg(), args, |_| {}, std::time::Duration::from_secs(60))
            .await
            .expect("rodar o ffmpeg");
    assert!(result.success, "{}", result.stderr);
    out
}

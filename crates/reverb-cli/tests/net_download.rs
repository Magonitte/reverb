//! F03 — T13–T17 (rede): o `reverb-cli` de verdade contra o YouTube, com as ferramentas reais de
//! `.test-tools/`. Rodam com `npm run verify:net`; usam diretórios temporários (protocolo §7, regra 11).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FX1: &str = "https://www.youtube.com/watch?v=jNQXAC9IVRw";
const FX2: &str = "https://music.youtube.com/watch?v=lYBUbBu4W08";
const FX4: &str = "https://www.youtube.com/playlist?list=OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE";
const FX5: &str = "https://music.youtube.com/browse/MPREb_dcYZhAh5urI";

fn tools_root() -> PathBuf {
    std::env::var_os("REVERB_TEST_TOOLS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tools"))
}

fn ffprobe() -> PathBuf {
    let root = tools_root();
    let manifest = std::fs::read_to_string(root.join("manifest.json")).unwrap_or_else(|_| {
        panic!(
            "ferramentas ausentes em {}: rode `npm run test:prepare`",
            root.display()
        )
    });
    let json: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    let version = json["tools"]["ffmpeg"]["current"]["version"]
        .as_str()
        .expect("ffmpeg ausente: rode `npm run test:prepare`");
    root.join("ffmpeg").join(version).join(if cfg!(windows) {
        "ffprobe.exe"
    } else {
        "ffprobe"
    })
}

/// Roda o CLI com dados isolados e as ferramentas de teste. Reexecuta até 3 vezes se a rede falhar
/// (protocolo §3: re-execução de testes @network).
fn cli(data_dir: &Path, args: &[&str]) -> Output {
    let mut last = None;
    for attempt in 1..=3 {
        let output = Command::new(env!("CARGO_BIN_EXE_reverb-cli"))
            .arg("--data-dir")
            .arg(data_dir)
            .arg("--tools-dir")
            .arg(tools_root())
            .args(args)
            .output()
            .expect("executar o reverb-cli");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let network_flake = !output.status.success()
            && (stderr.contains("(network)") || stderr.contains("(unknown)"));
        if output.status.success() || !network_flake {
            return output;
        }
        eprintln!("tentativa {attempt} falhou por rede: {stderr}");
        last = Some(output);
    }
    last.unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn probe(file: &Path) -> serde_json::Value {
    let output = Command::new(ffprobe())
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(file)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn files_in(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect()
}

fn download(profile: &str) -> (tempfile::TempDir, PathBuf, Output) {
    let temp = tempfile::tempdir().unwrap();
    let out = temp.path().join("Música Teste ç");
    let data = temp.path().join("dados");
    let output = cli(
        &data,
        &[
            "download",
            FX1,
            "--profile",
            profile,
            "--out",
            out.to_str().unwrap(),
        ],
    );
    (temp, out, output)
}

#[test]
#[ignore = "network"]
fn t13_download_original_gera_opus_e_deixa_o_tmp_vazio() {
    let (temp, out, output) = download("original");
    assert!(output.status.success(), "{}", stderr(&output));

    let files = files_in(&out);
    assert_eq!(files.len(), 1, "{files:?}");
    let file = &files[0];
    assert_eq!(file.extension().unwrap(), "opus");
    assert!(file.metadata().unwrap().len() > 100 * 1024);
    let last_line = stdout(&output).lines().last().unwrap().to_string();
    assert_eq!(
        Path::new(&last_line),
        file.as_path(),
        "imprime o caminho final"
    );

    let info = probe(file);
    let duration: f64 = info["format"]["duration"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((duration - 19.0).abs() <= 1.0, "{duration}");

    let tmp = temp.path().join("dados/tmp");
    assert!(
        !tmp.exists() || files_in(&tmp).is_empty(),
        "tmp deve ficar vazio"
    );
}

#[test]
#[ignore = "network"]
fn t14_download_mp3_v0_converte() {
    let (temp, out, output) = download("mp3_v0");
    assert!(output.status.success(), "{}", stderr(&output));
    let files = files_in(&out);
    assert_eq!(files.len(), 1, "{files:?}");
    assert_eq!(files[0].extension().unwrap(), "mp3");
    let info = probe(&files[0]);
    assert_eq!(info["streams"][0]["codec_name"], "mp3");
    let tmp = temp.path().join("dados/tmp");
    assert!(!tmp.exists() || files_in(&tmp).is_empty());
}

#[test]
#[ignore = "network"]
fn t15_analyze_faixa_oficial_album_e_url_do_youtube_music() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path();

    let output = cli(data, &["analyze", FX2, "--json"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(json["type"], "video");
    let info = &json["info"];
    assert_eq!(info["track"], "Never Gonna Give You Up");
    assert_eq!(info["artist"], "Rick Astley");
    assert_eq!(info["album"], "Whenever You Need Somebody");
    assert_eq!(info["isOfficialTrack"], true);

    for url in [FX4, FX5] {
        let output = cli(data, &["analyze", url, "--json"]);
        assert!(output.status.success(), "{url}: {}", stderr(&output));
        let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
        assert_eq!(json["type"], "collection", "{url}");
        assert_eq!(
            json["info"]["entries"].as_array().unwrap().len(),
            10,
            "{url}"
        );
    }
}

#[test]
#[ignore = "network"]
fn t16_busca_no_youtube_music_acha_a_faixa_oficial() {
    let temp = tempfile::tempdir().unwrap();
    let output = cli(
        temp.path(),
        &[
            "search",
            "rick astley never gonna give you up",
            "--source",
            "ytmusic",
        ],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let first = stdout(&output).lines().next().unwrap().to_string();
    assert!(first.starts_with("lYBUbBu4W08"), "{first}");
}

#[test]
#[ignore = "network"]
fn t17_url_inexistente_falha_como_unavailable() {
    let temp = tempfile::tempdir().unwrap();
    let out = temp.path().join("saida");
    let output = cli(
        &temp.path().join("dados"),
        &[
            "download",
            "https://www.youtube.com/watch?v=aaaaaaaaaaa",
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("(unavailable)"),
        "{}",
        stderr(&output)
    );
    assert!(!out.exists() || files_in(&out).is_empty());
}

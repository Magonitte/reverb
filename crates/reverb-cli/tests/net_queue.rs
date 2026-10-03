//! F04 — T15 (rede): `reverb-cli queue add` + `queue run --until-idle` com o YouTube de verdade.
//! Rodam com `npm run verify:net`; usam diretórios temporários e as ferramentas de `.test-tools/`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FX1: &str = "https://www.youtube.com/watch?v=jNQXAC9IVRw";

fn tools_root() -> PathBuf {
    std::env::var_os("REVERB_TEST_TOOLS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tools"))
}

fn ffprobe() -> PathBuf {
    let root = tools_root();
    let manifest = std::fs::read_to_string(root.join("manifest.json"))
        .expect("ferramentas ausentes: rode `npm run test:prepare`");
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

fn cli(data_dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_reverb-cli"))
        .arg("--data-dir")
        .arg(data_dir)
        .arg("--tools-dir")
        .arg(tools_root())
        .args(args)
        .output()
        .expect("executar o reverb-cli")
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

fn audio_files(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .unwrap()
        .flat_map(|entry| {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                audio_files(&entry.path())
            } else if matches!(
                entry.path().extension().and_then(|ext| ext.to_str()),
                Some("opus" | "mp3" | "m4a" | "flac" | "ogg" | "wav")
            ) {
                vec![entry.path()]
            } else {
                Vec::new()
            }
        })
        .collect()
}

#[test]
#[ignore = "network"]
fn t15_fila_baixa_tres_perfis_em_paralelo_e_deixa_o_tmp_vazio() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("dados");
    let out = temp.path().join("Música Teste ç");

    for (key, value) in [("outputDir", out.to_str().unwrap()), ("parallelism", "2")] {
        let output = cli(&data, &["settings", "set", key, value]);
        assert!(output.status.success(), "{}", stderr(&output));
    }
    for profile in ["original", "mp3_v0", "opus_96"] {
        let output = cli(&data, &["queue", "add", FX1, "--profile", profile]);
        assert!(output.status.success(), "{profile}: {}", stderr(&output));
    }

    let run = cli(&data, &["queue", "run", "--until-idle"]);
    assert!(run.status.success(), "{}", stderr(&run));

    let list = cli(&data, &["jobs", "list", "--json"]);
    let jobs: Vec<serde_json::Value> = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(jobs.len(), 3);
    assert!(jobs.iter().all(|j| j["status"] == "done"), "{jobs:#?}");

    // F09 organiza a fila em subpastas e escreve sidecars; ainda devem existir exatamente
    // os três áudios, associados à biblioteca e com os mesmos codecs/tamanhos do T15.
    let mut files: Vec<_> = audio_files(&out)
        .iter()
        .map(|path| reverb_core::library::files::canonical(path).unwrap())
        .collect();
    files.sort();
    assert_eq!(files.len(), 3, "{files:?}");
    let mut recorded: Vec<PathBuf> = jobs
        .iter()
        .map(|job| {
            assert!(
                job["libraryId"].as_i64().is_some_and(|id| id > 0),
                "{job:#?}"
            );
            PathBuf::from(job["outputPath"].as_str().unwrap())
        })
        .collect();
    recorded.sort();
    assert_eq!(files, recorded);
    let organized = reverb_core::library::files::canonical(&out.join("Outros/jawed")).unwrap();
    assert!(files.iter().all(|file| file.starts_with(&organized)));
    let mut codecs: Vec<String> = files
        .iter()
        .map(|f| {
            probe(f)["streams"][0]["codec_name"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    codecs.sort();
    assert_eq!(codecs, ["mp3", "opus", "opus"]);
    for file in &files {
        assert!(file.metadata().unwrap().len() > 50 * 1024, "{file:?}");
    }
    let tmp = data.join("tmp");
    assert!(
        !tmp.exists() || std::fs::read_dir(&tmp).unwrap().next().is_none(),
        "tmp deve ficar vazio"
    );
}

use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_reverb-cli"))
}

#[test]
fn doctor_imprime_json_com_as_chaves_esperadas() {
    let dir = tempfile::tempdir().unwrap();
    let out = cli()
        .args(["--data-dir"])
        .arg(dir.path())
        .arg("doctor")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout deve ser JSON");
    for key in ["version", "os", "arch", "dataDir", "portable"] {
        assert!(json.get(key).is_some(), "falta a chave {key}");
    }
    assert_eq!(json["portable"], false);
    assert_eq!(json["dataDir"], dir.path().to_string_lossy().as_ref());
}

#[test]
fn version_funciona() {
    let out = cli().arg("--version").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("reverb-cli"));
}

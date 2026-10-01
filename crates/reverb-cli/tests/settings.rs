use std::path::Path;
use std::process::{Command, Output};

fn cli(data_dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_reverb-cli"))
        .arg("--data-dir")
        .arg(data_dir)
        .args(args)
        .output()
        .unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn t15_set_e_get_devolvem_o_valor_gravado() {
    let dir = tempfile::tempdir().unwrap();

    let out = cli(dir.path(), &["settings", "set", "parallelism", "3"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = cli(dir.path(), &["settings", "get", "parallelism"]);
    assert!(out.status.success());
    assert_eq!(stdout(&out), "3");

    let out = cli(dir.path(), &["settings", "get"]);
    let json: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(json["parallelism"], 3);
    assert_eq!(json["queueLimit"], 500);
    assert!(json.get("acoustidKey").is_none());
}

#[test]
fn set_aceita_texto_simples_para_strings_e_json_para_listas() {
    let dir = tempfile::tempdir().unwrap();
    assert!(cli(dir.path(), &["settings", "set", "theme", "light"])
        .status
        .success());
    assert!(cli(
        dir.path(),
        &[
            "settings",
            "set",
            "sponsorblockCategories",
            r#"["intro","outro"]"#
        ]
    )
    .status
    .success());
    assert_eq!(
        stdout(&cli(dir.path(), &["settings", "get", "theme"])),
        "\"light\""
    );
    assert_eq!(
        stdout(&cli(
            dir.path(),
            &["settings", "get", "sponsorblockCategories"]
        )),
        r#"["intro","outro"]"#
    );
}

#[test]
fn valor_invalido_e_chave_desconhecida_falham() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!cli(dir.path(), &["settings", "set", "parallelism", "9"])
        .status
        .success());
    assert!(!cli(dir.path(), &["settings", "set", "naoExiste", "1"])
        .status
        .success());
    assert!(!cli(dir.path(), &["settings", "get", "naoExiste"])
        .status
        .success());
    // Segredos nunca são lidos pelo CLI.
    assert!(
        cli(dir.path(), &["settings", "set", "acoustidKey", "abc12345"])
            .status
            .success()
    );
    assert!(!cli(dir.path(), &["settings", "get", "acoustidKey"])
        .status
        .success());
}

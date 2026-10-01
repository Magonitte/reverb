//! T8: matriz de `resolve_js_runtime` com PATH falso (nunca altera o ambiente do processo).

use std::ffi::OsString;

use tempfile::TempDir;

use super::*;
use crate::tools::testutil::copy_fake_tool;

const DENO_OUT: &str =
    "deno 2.9.7 (stable, release, x86_64-pc-windows-msvc)\nv8 14.5.201.2-rusty\ntypescript 5.9.2";

/// Cria uma pasta (com espaço no nome) contendo `fake-tool` renomeado e a versão que ele imprime.
fn fake_path(entries: &[(&str, &str)]) -> (TempDir, OsString) {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("pasta no PATH");
    for (name, version) in entries {
        let exe = bin.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        copy_fake_tool(&exe);
        let mut sidecar = exe.into_os_string();
        sidecar.push(".version");
        std::fs::write(sidecar, version).unwrap();
    }
    std::fs::create_dir_all(&bin).unwrap();
    let path_var = std::env::join_paths([bin]).unwrap();
    (dir, path_var)
}

fn found(detection: Detection) -> JsRuntimeChoice {
    match detection {
        Detection::Found(choice) => choice,
        other => panic!("esperava Found, veio {other:?}"),
    }
}

fn file_stem(choice: &JsRuntimeChoice) -> String {
    choice
        .path
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

#[tokio::test]
async fn auto_prefere_o_deno_do_sistema() {
    let (_dir, path) = fake_path(&[("deno", DENO_OUT), ("node", "v22.1.0")]);
    let choice = found(
        detect_js_runtime(JsRuntime::Auto, &path, None)
            .await
            .unwrap(),
    );
    assert_eq!(choice.kind, JsKind::Deno);
    assert_eq!(file_stem(&choice), "deno");
    assert!(choice.js_runtime_arg().starts_with("deno:"));
    assert!(choice
        .js_runtime_arg()
        .ends_with(&choice.path.display().to_string()));
}

#[tokio::test]
async fn auto_usa_node_22_quando_nao_ha_deno() {
    let (_dir, path) = fake_path(&[("node", "v22.1.0")]);
    let choice = found(
        detect_js_runtime(JsRuntime::Auto, &path, None)
            .await
            .unwrap(),
    );
    assert_eq!(choice.kind, JsKind::Node);
    assert!(choice.js_runtime_arg().starts_with("node:"));
}

#[tokio::test]
async fn auto_rejeita_node_18_e_cai_no_deno_gerenciado() {
    let (_dir, path) = fake_path(&[("node", "v18.19.0")]);
    assert_eq!(
        detect_js_runtime(JsRuntime::Auto, &path, None)
            .await
            .unwrap(),
        Detection::NeedManagedDeno
    );
    let managed = PathBuf::from("/gerenciado/deno");
    let choice = found(
        detect_js_runtime(JsRuntime::Auto, &path, Some(managed.clone()))
            .await
            .unwrap(),
    );
    assert_eq!(
        choice,
        JsRuntimeChoice {
            kind: JsKind::Deno,
            path: managed
        }
    );
}

#[tokio::test]
async fn auto_sem_nada_precisa_do_deno_gerenciado() {
    let (_dir, path) = fake_path(&[]);
    assert_eq!(
        detect_js_runtime(JsRuntime::Auto, &path, None)
            .await
            .unwrap(),
        Detection::NeedManagedDeno
    );
    // PATH vazio.
    assert_eq!(
        detect_js_runtime(JsRuntime::Auto, OsStr::new(""), None)
            .await
            .unwrap(),
        Detection::NeedManagedDeno
    );
}

#[tokio::test]
async fn system_node_aceita_22_e_rejeita_18() {
    let (_dir, ok) = fake_path(&[("node", "v22.1.0")]);
    let choice = found(
        detect_js_runtime(JsRuntime::SystemNode, &ok, None)
            .await
            .unwrap(),
    );
    assert_eq!(choice.kind, JsKind::Node);

    let (_dir, old) = fake_path(&[("node", "v18.19.0")]);
    let err = detect_js_runtime(JsRuntime::SystemNode, &old, None)
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "js_runtime_missing");

    let (_dir, none) = fake_path(&[]);
    assert!(detect_js_runtime(JsRuntime::SystemNode, &none, None)
        .await
        .is_err());

    // Limite: exatamente 20 é aceito.
    let (_dir, edge) = fake_path(&[("node", "v20.0.0")]);
    assert!(detect_js_runtime(JsRuntime::SystemNode, &edge, None)
        .await
        .is_ok());
}

#[tokio::test]
async fn system_deno_exige_deno_2_ou_mais() {
    let (_dir, ok) = fake_path(&[("deno", DENO_OUT)]);
    assert_eq!(
        found(
            detect_js_runtime(JsRuntime::SystemDeno, &ok, None)
                .await
                .unwrap()
        )
        .kind,
        JsKind::Deno
    );
    let (_dir, old) = fake_path(&[(
        "deno",
        "deno 1.46.3 (stable, release, x86_64-pc-windows-msvc)",
    )]);
    assert_eq!(
        detect_js_runtime(JsRuntime::SystemDeno, &old, None)
            .await
            .unwrap_err()
            .kind(),
        "js_runtime_missing"
    );
    let (_dir, none) = fake_path(&[("node", "v22.1.0")]);
    assert!(detect_js_runtime(JsRuntime::SystemDeno, &none, None)
        .await
        .is_err());
}

#[tokio::test]
async fn managed_deno_ignora_o_sistema() {
    let (_dir, path) = fake_path(&[("deno", DENO_OUT), ("node", "v22.1.0")]);
    assert_eq!(
        detect_js_runtime(JsRuntime::ManagedDeno, &path, None)
            .await
            .unwrap(),
        Detection::NeedManagedDeno
    );
    let managed = PathBuf::from("/gerenciado/deno");
    let choice = found(
        detect_js_runtime(JsRuntime::ManagedDeno, &path, Some(managed.clone()))
            .await
            .unwrap(),
    );
    assert_eq!(choice.path, managed);
}

#[tokio::test]
async fn binario_do_sistema_que_nao_executa_e_ignorado() {
    // Um `deno` que sai com erro não conta.
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    let exe = bin.join(format!("deno{}", std::env::consts::EXE_SUFFIX));
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(&exe, "isto não é um executável").unwrap();
    let path = std::env::join_paths([bin]).unwrap();
    assert_eq!(
        detect_js_runtime(JsRuntime::Auto, &path, None)
            .await
            .unwrap(),
        Detection::NeedManagedDeno
    );
}

#[test]
fn which_in_procura_so_nos_diretorios_recebidos() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let exe = format!("ferramenta{}", if cfg!(windows) { ".exe" } else { "" });
    std::fs::write(b.join(&exe), "x").unwrap();

    let path = std::env::join_paths([&a, &b]).unwrap();
    assert_eq!(which_in("ferramenta", &path), Some(b.join(&exe)));
    assert_eq!(which_in("ferramenta", OsStr::new("")), None);
    assert_eq!(which_in("outra", &path), None);
    // Primeiro diretório do PATH vence.
    std::fs::write(a.join(&exe), "x").unwrap();
    assert_eq!(which_in("ferramenta", &path), Some(a.join(&exe)));
}

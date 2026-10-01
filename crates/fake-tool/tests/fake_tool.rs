use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn tool() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fake-tool"))
}

#[test]
fn version_padrao_e_via_env() {
    let out = tool()
        .arg("--version")
        .env_remove("FAKE_TOOL_VERSION")
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "1.0.0");

    let out = tool()
        .arg("--version")
        .env("FAKE_TOOL_VERSION", "2026.08.19")
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "2026.08.19");
}

#[test]
fn exit_devolve_o_codigo() {
    let out = tool().args(["--exit", "3"]).output().unwrap();
    assert_eq!(out.status.code(), Some(3));
}

#[test]
fn stderr_escreve_o_texto() {
    let out = tool()
        .args(["--stderr", "ERROR: boom", "--exit", "1"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("ERROR: boom"));
}

#[test]
fn stdout_lines_ecoa_o_arquivo() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("saida.txt");
    std::fs::write(&file, "um\ndois\ntrês\n").unwrap();
    let out = tool().arg("--stdout-lines").arg(&file).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "um\ndois\ntrês\n");
}

#[test]
fn sleep_demora_o_tempo_pedido() {
    let t0 = Instant::now();
    let out = tool().args(["--sleep", "0.3"]).output().unwrap();
    assert!(out.status.success());
    assert!(t0.elapsed() >= Duration::from_millis(280));
}

#[test]
fn spawn_child_sleep_imprime_o_pid_do_filho() {
    let mut child = tool()
        .args(["--spawn-child-sleep", "1"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut buf = [0u8; 64];
    let n = child.stdout.as_mut().unwrap().read(&mut buf).unwrap();
    let text = String::from_utf8_lossy(&buf[..n]).to_string();
    assert!(text.starts_with("CHILD_PID="), "saída inesperada: {text}");
    let pid: u32 = text
        .trim()
        .trim_start_matches("CHILD_PID=")
        .parse()
        .unwrap();
    assert!(pid > 0);
    child.wait().unwrap();
}

#[test]
fn http_ping_responde_versao() {
    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let mut child = tool()
        .args(["--http-ping", &port.to_string()])
        .spawn()
        .unwrap();

    let mut response = String::new();
    for _ in 0..50 {
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
            stream
                .write_all(b"GET /ping HTTP/1.1\r\nHost: x\r\n\r\n")
                .unwrap();
            stream.read_to_string(&mut response).unwrap();
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    child.kill().unwrap();
    child.wait().unwrap();

    assert!(response.starts_with("HTTP/1.1 200"), "resposta: {response}");
    assert!(response.contains(r#"{"version":"fake"}"#));
}

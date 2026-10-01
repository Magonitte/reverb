//! T9: a morte da árvore de processos, e a captura de saída.

use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessStatus, ProcessesToUpdate, System};
use tokio::io::{AsyncBufReadExt, BufReader};

use super::*;
use crate::tools::testutil::fake_tool_path;

fn is_alive(pid: u32) -> bool {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    // Zumbi (morto, ainda não colhido pelo init do contêiner) conta como morto.
    system
        .process(Pid::from_u32(pid))
        .is_some_and(|p| p.status() != ProcessStatus::Zombie)
}

async fn wait_dead(pids: &[u32], limit: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() < limit {
        if pids.iter().all(|pid| !is_alive(*pid)) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

#[tokio::test]
async fn matar_encerra_o_pai_e_o_filho_em_ate_3s() {
    let mut command = tool_command(fake_tool_path(), ["--spawn-child-sleep", "60"]);
    command.command_mut().stdout(Stdio::piped());
    let mut child = spawn_tool(command).unwrap();
    let parent_pid = child.id().expect("pid do pai");

    let stdout = child.stdout().take().expect("stdout");
    let mut lines = BufReader::new(stdout).lines();
    let line = tokio::time::timeout(Duration::from_secs(10), lines.next_line())
        .await
        .expect("o pai deveria imprimir o PID do filho")
        .unwrap()
        .expect("linha CHILD_PID");
    let child_pid: u32 = line
        .strip_prefix("CHILD_PID=")
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(is_alive(parent_pid) && is_alive(child_pid));

    let started = Instant::now();
    kill_tree(child.as_mut()).await.unwrap();
    assert!(
        wait_dead(&[parent_pid, child_pid], Duration::from_secs(3)).await,
        "pai ou filho continuam vivos {:?} após kill_tree",
        started.elapsed()
    );
}

#[tokio::test]
async fn largar_o_processo_tambem_mata_a_arvore() {
    let mut command = tool_command(fake_tool_path(), ["--spawn-child-sleep", "60"]);
    command.command_mut().stdout(Stdio::piped());
    let mut child = spawn_tool(command).unwrap();
    let parent_pid = child.id().unwrap();
    let stdout = child.stdout().take().unwrap();
    let mut lines = BufReader::new(stdout).lines();
    let line = lines.next_line().await.unwrap().unwrap();
    let child_pid: u32 = line
        .strip_prefix("CHILD_PID=")
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    drop(lines);
    drop(child); // kill-on-drop

    assert!(wait_dead(&[parent_pid, child_pid], Duration::from_secs(5)).await);
}

#[tokio::test]
async fn run_capture_junta_stdout_stderr_e_codigo() {
    let out = run_capture(
        fake_tool_path(),
        ["--version", "--stderr", "aviso", "--exit", "3"],
        |command| {
            command.env("FAKE_TOOL_VERSION", "9.8.7");
        },
        Duration::from_secs(10),
    )
    .await
    .unwrap();
    assert!(!out.success);
    assert_eq!(out.code, Some(3));
    assert_eq!(out.stdout.trim(), "9.8.7");
    assert_eq!(out.stderr.trim(), "aviso");
}

#[tokio::test]
async fn run_capture_respeita_o_tempo_limite() {
    let started = Instant::now();
    let err = run_capture(
        fake_tool_path(),
        ["--sleep", "30"],
        |_| {},
        Duration::from_millis(500),
    )
    .await
    .unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[tokio::test]
async fn argumentos_com_espacos_chegam_intactos() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("pasta com espaços").join("linhas.txt");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "primeira\nsegunda\n").unwrap();
    let out = run_capture(
        fake_tool_path(),
        [std::ffi::OsStr::new("--stdout-lines"), file.as_os_str()],
        |_| {},
        Duration::from_secs(10),
    )
    .await
    .unwrap();
    assert!(out.success, "{}", out.stderr);
    assert_eq!(
        out.stdout.lines().collect::<Vec<_>>(),
        ["primeira", "segunda"]
    );
}

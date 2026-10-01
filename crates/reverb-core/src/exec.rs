//! Execução de um processo externo lendo a saída **linha a linha**, com cancelamento, watchdog
//! de silêncio e tempo total. Usada pelo runner do yt-dlp e pela conversão com ffmpeg.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use crate::tools::process::{kill_tree, spawn_tool, tool_command};

/// Linhas de stderr guardadas para classificar o erro e registrar no log.
pub const STDERR_TAIL_LINES: usize = 200;
/// Depois que o processo principal sai, espera um pouco por linhas ainda em trânsito.
const DRAIN_GRACE: Duration = Duration::from_millis(500);
const POLL_EXIT: Duration = Duration::from_millis(50);

pub struct ExecSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    /// Sem nenhuma saída por esse tempo ⇒ mata a árvore (`ExecFailure::Idle`).
    pub idle_timeout: Option<Duration>,
    /// Tempo total máximo (`ExecFailure::Timeout`).
    pub total_timeout: Option<Duration>,
}

#[derive(Debug)]
pub enum ExecFailure {
    Cancelled,
    Idle,
    Timeout,
    Spawn(std::io::Error),
}

#[derive(Debug, Clone)]
pub struct ExecOutcome {
    pub success: bool,
    pub code: Option<i32>,
    pub stderr_tail: Vec<String>,
}

enum Line {
    Out(String),
    Err(String),
}

/// Lê linhas tolerando UTF-8 inválido (conversão com perdas) e fins de linha `\r\n`.
async fn pump(reader: impl AsyncRead + Unpin, stderr: bool, tx: mpsc::UnboundedSender<Line>) {
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let text = String::from_utf8_lossy(&buffer);
                let text = text.trim_end_matches(['\n', '\r']).to_string();
                let line = if stderr {
                    Line::Err(text)
                } else {
                    Line::Out(text)
                };
                if tx.send(line).is_err() {
                    break;
                }
            }
        }
    }
}

/// Executa e entrega cada linha do stdout a `on_stdout`. Cancelar, estourar um dos tempos ou
/// descartar a future mata a árvore inteira de processos.
pub async fn run_streaming(
    spec: ExecSpec,
    cancel: &CancellationToken,
    on_stdout: &mut (dyn FnMut(&str) + Send),
) -> Result<ExecOutcome, ExecFailure> {
    let mut wrap = tool_command(&spec.program, &spec.args);
    {
        let command = wrap.command_mut();
        command
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        for (key, value) in &spec.env {
            command.env(key, value);
        }
    }
    let mut child = spawn_tool(wrap).map_err(ExecFailure::Spawn)?;

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut readers = Vec::new();
    if let Some(stdout) = child.stdout().take() {
        readers.push(tokio::spawn(pump(stdout, false, tx.clone())));
    }
    if let Some(stderr) = child.stderr().take() {
        readers.push(tokio::spawn(pump(stderr, true, tx.clone())));
    }
    drop(tx);

    let started = Instant::now();
    let mut last_output = Instant::now();
    let mut stderr_tail: VecDeque<String> = VecDeque::with_capacity(STDERR_TAIL_LINES);
    let mut exit = None;
    let mut exited_at: Option<Instant> = None;
    let mut poll = tokio::time::interval(POLL_EXIT);

    let result = loop {
        let idle_deadline = spec
            .idle_timeout
            .map(|t| last_output + t)
            .unwrap_or_else(|| Instant::now() + Duration::from_secs(86_400));
        let total_deadline = spec
            .total_timeout
            .map(|t| started + t)
            .unwrap_or_else(|| Instant::now() + Duration::from_secs(86_400));

        tokio::select! {
            biased;
            _ = cancel.cancelled() => break Err(ExecFailure::Cancelled),
            line = rx.recv() => match line {
                None => break Ok(()),
                Some(line) => {
                    last_output = Instant::now();
                    match line {
                        Line::Out(text) => on_stdout(&text),
                        Line::Err(text) => {
                            tracing::debug!(target: "yt", stderr = %text);
                            if stderr_tail.len() == STDERR_TAIL_LINES {
                                stderr_tail.pop_front();
                            }
                            stderr_tail.push_back(text);
                        }
                    }
                }
            },
            _ = tokio::time::sleep_until(idle_deadline), if spec.idle_timeout.is_some() => {
                break Err(ExecFailure::Idle)
            }
            _ = tokio::time::sleep_until(total_deadline), if spec.total_timeout.is_some() => {
                break Err(ExecFailure::Timeout)
            }
            _ = poll.tick() => {
                if exit.is_none() {
                    if let Ok(Some(status)) = child.try_wait() {
                        exit = Some(status);
                        exited_at = Some(Instant::now());
                    }
                } else if exited_at.is_some_and(|at| at.elapsed() >= DRAIN_GRACE) {
                    // Algum neto ainda segura os pipes: o que importa já foi lido.
                    break Ok(());
                }
            }
        }
    };

    if let Err(failure) = result {
        let _ = kill_tree(child.as_mut()).await;
        for reader in readers {
            reader.abort();
        }
        return Err(failure);
    }

    // Pipes fechados (ou prazo de dreno vencido): colhe o status e limpa quem sobrou na árvore.
    let status = match exit {
        Some(status) => status,
        None => tokio::select! {
            biased;
            _ = cancel.cancelled() => {
                let _ = kill_tree(child.as_mut()).await;
                return Err(ExecFailure::Cancelled);
            }
            status = child.wait() => status.map_err(ExecFailure::Spawn)?,
        },
    };
    let _ = kill_tree(child.as_mut()).await;
    for reader in readers {
        reader.abort();
    }
    Ok(ExecOutcome {
        success: status.success(),
        code: status.code(),
        stderr_tail: stderr_tail.into(),
    })
}

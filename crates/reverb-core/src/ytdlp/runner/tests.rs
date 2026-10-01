//! T6–T9 — o runner com o `fake-tool` no lugar do yt-dlp (`FAKE_TOOL_OPTS` controla o falso).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessesToUpdate, System};

use super::*;
use crate::tools::testutil::{fake_tool_path, fixture};

fn ctx() -> YtDlpContext {
    YtDlpContext {
        ytdlp_path: fake_tool_path(),
        js_runtime_arg: "deno:/nada/deno".to_string(),
        ffmpeg_dir: PathBuf::from("/nada/ffmpeg"),
        cookies: None,
        limit_rate_mbps: None,
        pot_args: None,
    }
}

fn runner_with(opts: &[&str], tweak: impl FnOnce(&mut RunnerConfig)) -> YtDlpRunner {
    let mut config = RunnerConfig {
        extra_env: vec![("FAKE_TOOL_OPTS".to_string(), opts.join("\n"))],
        ..RunnerConfig::default()
    };
    tweak(&mut config);
    YtDlpRunner::with_config(None, config)
}

fn runner(opts: &[&str]) -> YtDlpRunner {
    runner_with(opts, |_| {})
}

fn options() -> DownloadOptions {
    DownloadOptions {
        url: "https://www.youtube.com/watch?v=jNQXAC9IVRw".to_string(),
        tmp_dir: PathBuf::from("/nada/tmp"),
        sponsorblock: None,
    }
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Grava um JSON compacto (uma linha) para o falso imprimir rápido.
fn compact_json(dir: &Path, name: &str, fixture_name: &str) -> String {
    let text = std::fs::read_to_string(fixture(&format!("ytdlp/{fixture_name}"))).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, serde_json::to_string(&value).unwrap()).unwrap();
    path_text(&path)
}

#[tokio::test]
async fn t6_progresso_na_ordem_e_done_correto() {
    let stdout = path_text(&fixture("ytdlp/stdout-download.txt"));
    let runner = runner(&["--stdout-lines", &stdout]);
    let updates = Mutex::new(Vec::new());
    let on_progress = |update: ProgressUpdate| updates.lock().unwrap().push(update);

    let done = runner
        .download(&ctx(), &options(), &CancellationToken::new(), &on_progress)
        .await
        .unwrap();

    let updates = updates.into_inner().unwrap();
    let downloaded: Vec<u64> = updates.iter().map(|u| u.downloaded).collect();
    assert_eq!(downloaded, [1024, 3072, 252182]);
    assert!(updates[2].finished && !updates[0].finished);
    assert_eq!(done.id, "jNQXAC9IVRw");
    assert_eq!(done.title, "Me at the zoo");
    assert_eq!(done.ext, "opus");
    assert_eq!(done.format_id.as_deref(), Some("251"));
}

#[tokio::test]
async fn sem_report_done_o_download_falha() {
    let runner = runner(&[]);
    let err = runner
        .download(&ctx(), &options(), &CancellationToken::new(), &|_| {})
        .await
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Unknown);
    assert!(err.message.contains("sem informar o arquivo"), "{err}");
}

#[tokio::test]
async fn t7_video_indisponivel_e_classificado_com_o_stderr_real() {
    let real = std::fs::read_to_string(fixture("ytdlp/stderr-unavailable.txt")).unwrap();
    let runner = runner(&["--exit", "1", "--stderr", real.trim_end()]);
    let err = runner
        .download(&ctx(), &options(), &CancellationToken::new(), &|_| {})
        .await
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Unavailable);
    assert!(err.message.contains("This video is unavailable"), "{err}");
    assert_eq!(err.stderr_tail.len(), 1);
}

#[tokio::test]
async fn t7_erro_de_rede_e_classificado() {
    let runner = runner(&["--exit", "1", "--stderr", "ERROR: Connection reset by peer"]);
    let err = runner
        .download(&ctx(), &options(), &CancellationToken::new(), &|_| {})
        .await
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Network);
    assert!(err.kind.is_retryable());
}

#[tokio::test]
async fn guarda_apenas_as_ultimas_200_linhas_do_stderr() {
    let runner = runner(&["--stderr-count", "260", "--exit", "1"]);
    let err = runner
        .download(&ctx(), &options(), &CancellationToken::new(), &|_| {})
        .await
        .unwrap_err();
    assert_eq!(err.stderr_tail.len(), 200);
    assert_eq!(err.stderr_tail.first().unwrap(), "linha 60");
    assert_eq!(err.stderr_tail.last().unwrap(), "linha 259");
}

#[tokio::test]
async fn t8_cancelamento_mata_a_arvore_em_ate_3_segundos() {
    let runner = runner(&["--spawn-child-sleep", "60"]);
    let cancel = CancellationToken::new();
    let pid = Arc::new(Mutex::new(None::<u32>));

    let task = {
        let pid = Arc::clone(&pid);
        let cancel = cancel.clone();
        let ctx = ctx();
        async move {
            let mut on_line = |line: &str| {
                if let Some(n) = line.strip_prefix("CHILD_PID=") {
                    *pid.lock().unwrap() = n.trim().parse().ok();
                }
            };
            runner
                .exec(&ctx, Vec::new(), None, None, &cancel, &mut on_line)
                .await
        }
    };
    let handle = tokio::spawn(task);

    // Espera o filho existir antes de cancelar.
    let started = Instant::now();
    while pid.lock().unwrap().is_none() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "o filho não apareceu"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let child_pid = pid.lock().unwrap().unwrap();
    let cancelled_at = Instant::now();
    cancel.cancel();

    let err = handle.await.unwrap().unwrap_err();
    assert_eq!(err.kind, ErrorKind::Cancelled);
    assert!(
        cancelled_at.elapsed() <= Duration::from_secs(3),
        "demorou {:?}",
        cancelled_at.elapsed()
    );

    // O neto não pode sobreviver.
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::All, true);
        if system.process(Pid::from_u32(child_pid)).is_none() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "o processo filho {child_pid} continua vivo"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test]
async fn cancelar_antes_de_comecar_retorna_cancelled() {
    let runner = runner(&["--sleep", "30"]);
    let cancel = CancellationToken::new();
    cancel.cancel();
    let started = Instant::now();
    let err = runner
        .download(&ctx(), &options(), &cancel, &|_| {})
        .await
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Cancelled);
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[tokio::test]
async fn t9_watchdog_mata_o_processo_silencioso_como_network() {
    let runner = runner_with(&["--sleep", "30"], |c| c.watchdog = Duration::from_secs(2));
    let started = Instant::now();
    let err = runner
        .download(&ctx(), &options(), &CancellationToken::new(), &|_| {})
        .await
        .unwrap_err();
    let elapsed = started.elapsed();
    assert_eq!(err.kind, ErrorKind::Network);
    assert!(err.message.contains("watchdog"), "{err}");
    assert!(
        elapsed >= Duration::from_secs(2) && elapsed < Duration::from_secs(8),
        "{elapsed:?}"
    );
}

#[tokio::test]
async fn saida_periodica_reinicia_o_watchdog() {
    // O falso imprime 5 linhas com 50 ms de intervalo; o watchdog de 1 s não deve disparar.
    let stdout = path_text(&fixture("ytdlp/stdout-download.txt"));
    let runner = runner_with(&["--stdout-lines", &stdout], |c| {
        c.watchdog = Duration::from_secs(1)
    });
    assert!(runner
        .download(&ctx(), &options(), &CancellationToken::new(), &|_| {})
        .await
        .is_ok());
}

#[tokio::test]
async fn analisar_video_le_o_json_do_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let json = compact_json(dir.path(), "fx2.json", "fx2-music.json");
    let runner = runner(&["--stdout-lines", &json]);
    let info = runner
        .analyze_video(
            &ctx(),
            "https://music.youtube.com/watch?v=lYBUbBu4W08",
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert!(info.is_official_track);
    assert_eq!(info.album.as_deref(), Some("Whenever You Need Somebody"));
}

#[tokio::test]
async fn analyze_despacha_por_tipo_de_url() {
    let dir = tempfile::tempdir().unwrap();
    let album = compact_json(dir.path(), "fx4.json", "fx4-album.json");
    let runner = runner(&["--stdout-lines", &album]);
    let url = "https://www.youtube.com/playlist?list=OLAK5uy_nmDUsWOMoEcz0SsVqUwir0oxu-k1oUyXE";
    match runner
        .analyze(&ctx(), url, &CancellationToken::new())
        .await
        .unwrap()
    {
        Analysis::Collection { info } => assert_eq!(info.entries.len(), 10),
        other => panic!("esperava coleção: {other:?}"),
    }

    let video = compact_json(dir.path(), "fx1.json", "fx1-video.json");
    let runner = self::runner(&["--stdout-lines", &video]);
    match runner
        .analyze(
            &ctx(),
            "https://youtu.be/jNQXAC9IVRw",
            &CancellationToken::new(),
        )
        .await
        .unwrap()
    {
        Analysis::Video { info } => assert_eq!(info.id, "jNQXAC9IVRw"),
        other => panic!("esperava vídeo: {other:?}"),
    }

    let err = runner
        .analyze(&ctx(), "https://vimeo.com/123", &CancellationToken::new())
        .await
        .unwrap_err();
    assert!(err.message.contains("não suportada"));
}

#[tokio::test]
async fn buscar_devolve_resultados() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("busca.json");
    std::fs::write(
        &file,
        r#"{"entries":[{"id":"lYBUbBu4W08","title":"Never Gonna Give You Up","url":"https://www.youtube.com/watch?v=lYBUbBu4W08"}]}"#,
    )
    .unwrap();
    let runner = runner(&["--stdout-lines", &path_text(&file)]);
    let results = runner
        .search(
            &ctx(),
            SearchSource::YtMusic,
            "rick astley",
            3,
            &CancellationToken::new(),
        )
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "lYBUbBu4W08");
}

#[tokio::test]
async fn json_invalido_vira_erro_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("lixo.txt");
    std::fs::write(&file, "isto não é json").unwrap();
    let runner = runner(&["--stdout-lines", &path_text(&file)]);
    let err = runner
        .analyze_video(
            &ctx(),
            "https://www.youtube.com/watch?v=jNQXAC9IVRw",
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Unknown);
}

#[tokio::test]
async fn analisar_respeita_o_tempo_total() {
    let runner = runner_with(&["--sleep", "30"], |c| {
        c.analyze_timeout = Duration::from_secs(1)
    });
    let started = Instant::now();
    let err = runner
        .analyze_video(
            &ctx(),
            "https://www.youtube.com/watch?v=jNQXAC9IVRw",
            &CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Network);
    assert!(started.elapsed() < Duration::from_secs(6));
}

#[tokio::test]
async fn programa_inexistente_e_erro_unknown() {
    let mut ctx = ctx();
    ctx.ytdlp_path = PathBuf::from("/nao/existe/yt-dlp");
    let err = YtDlpRunner::new(None)
        .download(&ctx, &options(), &CancellationToken::new(), &|_| {})
        .await
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Unknown);
    assert!(err.message.contains("não foi possível iniciar"), "{err}");
}

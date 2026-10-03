//! Pipeline com o `fake-tool` no lugar do yt-dlp: pasta temporária, cancelamento e destino final.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::*;
use crate::backend::{StaticContext, YtDlpProcessBackend};
use crate::profiles::profile;
use crate::tools::testutil::{fake_tool_path, fixture};
use crate::ytdlp::{RunnerConfig, YtDlpContext, YtDlpRunner};

fn pipeline(data: &Path, opts: &[&str]) -> DownloadPipeline {
    let ctx = YtDlpContext {
        ytdlp_path: fake_tool_path(),
        js_runtime_arg: "deno:/nada".to_string(),
        ffmpeg_dir: PathBuf::from("/nada/ffmpeg"),
        cookies: None,
        limit_rate_mbps: None,
        pot_args: None,
    };
    let config = RunnerConfig {
        extra_env: vec![("FAKE_TOOL_OPTS".to_string(), opts.join("\n"))],
        ..RunnerConfig::default()
    };
    let backend = YtDlpProcessBackend::new(
        YtDlpRunner::with_config(None, config),
        Arc::new(StaticContext(ctx)),
    );
    DownloadPipeline::new(
        Arc::new(backend),
        data.to_path_buf(),
        PathBuf::from("/nada/ffmpeg"),
        None,
    )
}

fn job(job_id: &str, out_dir: &Path) -> PipelineJob {
    PipelineJob {
        job_id: job_id.to_string(),
        url: "https://www.youtube.com/watch?v=jNQXAC9IVRw".to_string(),
        profile: profile("original").unwrap(),
        out_dir: out_dir.to_path_buf(),
        sponsorblock: None,
        metadata_override: None,
        fetch_metadata: None,
        settings: None,
        options: Default::default(),
        playlist_ctx: None,
    }
}

/// Cria `<data>/tmp/<job_id>/<name>` e um arquivo de stdout do falso com o `REVERB_DONE` dele.
fn stage_download(data: &Path, job_id: &str, title: &str, name: &str) -> PathBuf {
    let dir = data.join("tmp").join(job_id);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(name);
    std::fs::write(&file, b"audio de mentira").unwrap();
    let done = serde_json::json!({
        "id": "jNQXAC9IVRw", "title": title, "filepath": file, "ext": "opus",
        "abr": 106.0, "acodec": "opus", "format_id": "251", "duration": 19
    });
    let lines = data.join(format!("{job_id}.stdout"));
    std::fs::write(&lines, format!("REVERB_DONE {done}\n")).unwrap();
    lines
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[tokio::test]
async fn move_para_o_destino_com_nome_sanitizado_e_apaga_a_pasta_do_job() {
    let data = tempfile::tempdir().unwrap();
    let out = data.path().join("Música Teste ç");
    let stdout = stage_download(data.path(), "job-1", "Título: Teste?", "jNQXAC9IVRw.opus");
    let pipe = pipeline(data.path(), &["--stdout-lines", &text(&stdout)]);

    let result = pipe
        .run(&job("job-1", &out), &CancellationToken::new(), &|_| {})
        .await
        .unwrap();

    assert_eq!(result.path, out.join("Título - Teste_.opus"));
    assert_eq!(std::fs::read(&result.path).unwrap(), b"audio de mentira");
    assert!(
        !data.path().join("tmp/job-1").exists(),
        "pasta do job apagada"
    );
    assert_eq!(result.done.id, "jNQXAC9IVRw");
    assert!(!result.profile.needs_conversion());
}

#[tokio::test]
async fn arquivo_existente_ganha_sufixo() {
    let data = tempfile::tempdir().unwrap();
    let out = data.path().join("saida");
    for (n, expected) in [(1, "Faixa.opus"), (2, "Faixa (2).opus")] {
        let id = format!("job-{n}");
        let stdout = stage_download(data.path(), &id, "Faixa", "a.opus");
        let pipe = pipeline(data.path(), &["--stdout-lines", &text(&stdout)]);
        let result = pipe
            .run(&job(&id, &out), &CancellationToken::new(), &|_| {})
            .await
            .unwrap();
        assert_eq!(result.path, out.join(expected));
    }
}

#[tokio::test]
async fn erro_do_yt_dlp_apaga_a_pasta_do_job() {
    let data = tempfile::tempdir().unwrap();
    let real = std::fs::read_to_string(fixture("ytdlp/stderr-unavailable.txt")).unwrap();
    let pipe = pipeline(data.path(), &["--exit", "1", "--stderr", real.trim_end()]);
    let err = pipe
        .run(
            &job("job-e", &data.path().join("o")),
            &CancellationToken::new(),
            &|_| {},
        )
        .await
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Unavailable);
    assert!(!data.path().join("tmp/job-e").exists());
    assert!(!data.path().join("o").exists(), "nada é criado no destino");
}

#[tokio::test]
async fn done_apontando_para_arquivo_inexistente_e_erro() {
    let data = tempfile::tempdir().unwrap();
    let stdout = stage_download(data.path(), "job-x", "T", "a.opus");
    std::fs::remove_file(data.path().join("tmp/job-x/a.opus")).unwrap();
    let pipe = pipeline(data.path(), &["--stdout-lines", &text(&stdout)]);
    let err = pipe
        .run(
            &job("job-x", &data.path().join("o")),
            &CancellationToken::new(),
            &|_| {},
        )
        .await
        .unwrap_err();
    assert!(err.message.contains("não encontrado"), "{err}");
    assert!(!data.path().join("tmp/job-x").exists());
}

#[tokio::test]
async fn t8_cancelamento_apaga_a_pasta_do_job() {
    let data = tempfile::tempdir().unwrap();
    let pipe = pipeline(data.path(), &["--spawn-child-sleep", "60"]);
    let cancel = CancellationToken::new();
    let events = Mutex::new(0);

    let job_c = job("job-c", &data.path().join("o"));
    let on_event = |_| *events.lock().unwrap() += 1;
    let run = pipe.run(&job_c, &cancel, &on_event);
    let canceller = async {
        tokio::time::sleep(Duration::from_millis(700)).await;
        assert!(
            data.path().join("tmp/job-c").is_dir(),
            "a pasta existe durante o job"
        );
        cancel.cancel();
        Instant::now()
    };
    let (result, cancelled_at) = tokio::join!(run, canceller);

    let err = result.unwrap_err();
    assert_eq!(err.kind, ErrorKind::Cancelled);
    assert!(cancelled_at.elapsed() <= Duration::from_secs(3));
    assert!(
        !data.path().join("tmp/job-c").exists(),
        "pasta do job apagada"
    );
}

#[tokio::test]
async fn job_id_invalido_e_rejeitado() {
    let data = tempfile::tempdir().unwrap();
    let pipe = pipeline(data.path(), &[]);
    let err = pipe
        .run(
            &job("../fora", &data.path().join("o")),
            &CancellationToken::new(),
            &|_| {},
        )
        .await
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Disk);
}

/// Pasta do ffmpeg real de `.test-tools` (falha pedindo `npm run test:prepare` se não houver).
fn real_ffmpeg_dir() -> PathBuf {
    let root = std::env::var_os("REVERB_TEST_TOOLS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tools"));
    let manifest = std::fs::read_to_string(root.join("manifest.json"))
        .unwrap_or_else(|_| panic!("rode `npm run test:prepare` ({} ausente)", root.display()));
    let json: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    let version = json["tools"]["ffmpeg"]["current"]["version"]
        .as_str()
        .expect("ffmpeg ausente: rode `npm run test:prepare`");
    root.join("ffmpeg").join(version)
}

#[tokio::test]
async fn converte_com_o_ffmpeg_real_antes_de_mover() {
    let data = tempfile::tempdir().unwrap();
    let ffmpeg_dir = real_ffmpeg_dir();
    let tmp = data.path().join("tmp/job-m");
    std::fs::create_dir_all(&tmp).unwrap();
    let source = tmp.join("origem.opus");
    let status = std::process::Command::new(ffmpeg_dir.join(if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }))
    .args([
        "-hide_banner",
        "-nostdin",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "anoisesrc=color=pink:amplitude=0.3:duration=3",
        "-c:a",
        "libopus",
    ])
    .arg(&source)
    .output()
    .unwrap();
    assert!(status.status.success());

    let done = serde_json::json!({
        "id": "x", "title": "Faixa convertida", "filepath": source, "ext": "opus",
        "abr": null, "acodec": "opus", "format_id": "251", "duration": 3
    });
    let lines = data.path().join("m.stdout");
    std::fs::write(&lines, format!("REVERB_DONE {done}\n")).unwrap();

    let mut pipe = pipeline(data.path(), &["--stdout-lines", &text(&lines)]);
    pipe.ffmpeg_dir = ffmpeg_dir;
    let out = data.path().join("saida");
    let mut j = job("job-m", &out);
    j.profile = profile("mp3_v0").unwrap();
    let converted = Mutex::new(Vec::new());
    let result = pipe
        .run(&j, &CancellationToken::new(), &|event| {
            if let PipelineEvent::Convert(p) = event {
                converted.lock().unwrap().push(p);
            }
        })
        .await
        .unwrap();

    assert_eq!(result.path, out.join("Faixa convertida.mp3"));
    let bytes = std::fs::read(&result.path).unwrap();
    assert!(
        bytes.len() > 1000,
        "mp3 de verdade, não o arquivo de origem"
    );
    assert_ne!(&bytes[..4], b"OggS");
    assert_eq!(converted.into_inner().unwrap().last(), Some(&100));
    assert!(!data.path().join("tmp/job-m").exists());
}

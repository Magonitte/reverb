//! F09 T3b — ruído rosa real, +6 dB, análise não destrutiva e erros/cancelamento.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use reverb_core::loudness::analyze;
use reverb_core::tools::run_capture;
use reverb_core::ytdlp::ErrorKind;
use tokio_util::sync::CancellationToken;

async fn render(args: Vec<String>) {
    let output = run_capture(common::ffmpeg(), args, |_| {}, Duration::from_secs(60))
        .await
        .expect("rodar ffmpeg");
    assert!(output.success, "{}", output.stderr);
}

async fn pink_noise(dir: &Path) -> PathBuf {
    let file = dir.join("ruído rosa original.wav");
    let mut args = [
        "-hide_banner",
        "-nostdin",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "anoisesrc=color=pink:amplitude=0.1:duration=4:seed=1",
        "-c:a",
        "pcm_f32le",
    ]
    .map(String::from)
    .to_vec();
    args.push(file.to_string_lossy().into_owned());
    render(args).await;
    file
}

#[tokio::test]
async fn t3b_mesmo_ruido_com_volume_6db_reduz_ganho_em_6db_sem_alterar_arquivos() {
    let dir = tempfile::tempdir().unwrap();
    let original = pink_noise(dir.path()).await;
    let louder = dir.path().join("ruído rosa +6dB.wav");
    let mut args = ["-hide_banner", "-nostdin", "-y", "-i"]
        .map(String::from)
        .to_vec();
    args.push(original.to_string_lossy().into_owned());
    args.extend(["-af", "volume=6dB", "-c:a", "pcm_f32le"].map(String::from));
    args.push(louder.to_string_lossy().into_owned());
    render(args).await;

    let before_original = std::fs::read(&original).unwrap();
    let before_louder = std::fs::read(&louder).unwrap();
    let cancel = CancellationToken::new();
    let base = analyze(&common::ffmpeg(), &original, &cancel)
        .await
        .unwrap();
    let boosted = analyze(&common::ffmpeg(), &louder, &cancel).await.unwrap();
    let base_gain = base.replay_gain().unwrap();
    let boosted_gain = boosted.replay_gain().unwrap();
    let delta = base_gain.track_gain_db - boosted_gain.track_gain_db;
    assert!(
        (delta - 6.0).abs() <= 0.5,
        "base: {base:?}, boosted: {boosted:?}, delta: {delta}"
    );
    assert!((boosted.true_peak_dbfs - base.true_peak_dbfs - 6.0).abs() <= 0.2);
    assert_eq!(
        base_gain.r128_track_gain,
        ((-23.0 - base.integrated_lufs) * 256.0).round() as i16
    );
    assert_eq!(
        boosted_gain.r128_track_gain,
        ((-23.0 - boosted.integrated_lufs) * 256.0).round() as i16
    );
    assert_eq!(std::fs::read(&original).unwrap(), before_original);
    assert_eq!(std::fs::read(&louder).unwrap(), before_louder);
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        2,
        "análise não cria intermediários"
    );
}

#[tokio::test]
async fn arquivo_invalido_retorna_erro_ffmpeg_com_diagnostico() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("inválido.wav");
    std::fs::write(&file, "não é áudio").unwrap();
    let error = analyze(&common::ffmpeg(), &file, &CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Ffmpeg);
    assert!(!error.stderr_tail.is_empty());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "não é áudio");
}

#[tokio::test]
async fn cancelar_a_analise_retorna_cancelled() {
    let dir = tempfile::tempdir().unwrap();
    let source = pink_noise(dir.path()).await;
    let cancel = CancellationToken::new();
    cancel.cancel();
    let error = analyze(&common::ffmpeg(), &source, &cancel)
        .await
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Cancelled);
}

#[tokio::test]
async fn ffmpeg_ausente_retorna_erro_ffmpeg() {
    let dir = tempfile::tempdir().unwrap();
    let error = analyze(
        &dir.path().join("ffmpeg-inexistente"),
        &dir.path().join("input.wav"),
        &CancellationToken::new(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.kind, ErrorKind::Ffmpeg);
    assert!(error.message.contains("iniciar"));
}

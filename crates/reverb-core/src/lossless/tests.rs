use super::*;
use crate::test_tools::{ffmpeg, ffprobe};

#[test]
fn weak_treble_requires_an_abrupt_cutoff() {
    assert_eq!(
        verdict(Some(96000), -33.0, -99.0, -100.0, -2.0, -1.0).id(),
        "lossless"
    );
    assert_eq!(
        verdict(Some(44100), -10.0, -45.0, -60.0, -35.0, -30.0).id(),
        "lossy_16k"
    );
    assert_eq!(
        verdict(Some(44100), -10.0, -20.0, -60.0, -3.0, -30.0).id(),
        "lossy_19k"
    );
}

async fn generate(path: &Path, rate: u32) {
    let args = vec![
        "-hide_banner".into(),
        "-nostdin".into(),
        "-y".into(),
        "-f".into(),
        "lavfi".into(),
        "-i".into(),
        format!("anoisesrc=color=white:sample_rate={rate}:duration=8:seed=17"),
        "-f".into(),
        "lavfi".into(),
        "-i".into(),
        format!("anoisesrc=color=white:sample_rate={rate}:duration=8:seed=19"),
        "-filter_complex".into(),
        "[0:a][1:a]amerge=inputs=2".into(),
        "-c:a".into(),
        "flac".into(),
        path.to_string_lossy().into_owned(),
    ];
    crate::quality::audio::execute(&ffmpeg(), args, &CancellationToken::new())
        .await
        .unwrap();
}
#[tokio::test]
async fn lossless_noise_lossy_transcodes_and_low_sample_rate() {
    let tmp = tempfile::tempdir().unwrap();
    let original = tmp.path().join("original.flac");
    generate(&original, 44100).await;
    let result = verify(&ffmpeg(), &ffprobe(), &original, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(result.verdict.id(), "lossless", "{}", result.details);
    eprintln!("original {}", result.details);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(result.spectrogram_png_base64)
        .unwrap();
    assert_eq!(image::load_from_memory(&bytes).unwrap().width(), 800);
    assert_eq!(image::load_from_memory(&bytes).unwrap().height(), 300);
    for bitrate in [128, 320] {
        let mp3 = tmp.path().join(format!("lossy-{bitrate}.mp3"));
        let flac = tmp.path().join(format!("lossy-{bitrate}.flac"));
        crate::quality::audio::execute(
            &ffmpeg(),
            vec![
                "-y".into(),
                "-i".into(),
                original.to_string_lossy().into_owned(),
                "-c:a".into(),
                "libmp3lame".into(),
                "-b:a".into(),
                format!("{bitrate}k"),
                mp3.to_string_lossy().into_owned(),
            ],
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        crate::quality::audio::execute(
            &ffmpeg(),
            vec![
                "-y".into(),
                "-i".into(),
                mp3.to_string_lossy().into_owned(),
                "-c:a".into(),
                "flac".into(),
                flac.to_string_lossy().into_owned(),
            ],
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        let report = verify(&ffmpeg(), &ffprobe(), &flac, &CancellationToken::new())
            .await
            .unwrap();
        eprintln!("{bitrate} {}", report.details);
        if bitrate == 128 {
            assert_eq!(report.verdict.id(), "lossy_16k", "{}", report.details);
        } else {
            assert!(
                matches!(
                    report.verdict,
                    LosslessVerdict::Lossy16k | LosslessVerdict::Lossy19k
                ),
                "{}",
                report.details
            );
        }
    }
    let low = tmp.path().join("low.flac");
    generate(&low, 22050).await;
    assert_eq!(
        verify(&ffmpeg(), &ffprobe(), &low, &CancellationToken::new())
            .await
            .unwrap()
            .verdict
            .id(),
        "inconclusive"
    );
}

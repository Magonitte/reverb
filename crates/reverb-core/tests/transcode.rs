//! T10 — conversão com o ffmpeg real de `.test-tools`: um perfil por vez, conferindo codec,
//! bitrate e duração com o `ffprobe` e o progresso até 100 %.

mod common;

use std::sync::Mutex;

use reverb_core::profiles::profile;
use reverb_core::transcode::{convert, probe};
use reverb_core::ytdlp::ErrorKind;
use tokio_util::sync::CancellationToken;

use common::{ffmpeg, ffprobe, pink_noise_opus};

struct Expect {
    id: &'static str,
    codec: &'static str,
    ext: &'static str,
    bitrate: (u32, u32),
}

const CASES: &[Expect] = &[
    Expect {
        id: "mp3_320",
        codec: "mp3",
        ext: "mp3",
        bitrate: (310, 330),
    },
    Expect {
        id: "mp3_v0",
        codec: "mp3",
        ext: "mp3",
        bitrate: (180, 330),
    },
    Expect {
        id: "aac_256",
        codec: "aac",
        ext: "m4a",
        bitrate: (180, 300),
    },
    Expect {
        id: "opus_96",
        codec: "opus",
        ext: "opus",
        bitrate: (70, 115),
    },
    Expect {
        id: "flac",
        codec: "flac",
        ext: "flac",
        bitrate: (0, u32::MAX),
    },
];

#[tokio::test]
async fn t10_converte_cada_perfil_e_confere_com_o_ffprobe() {
    let dir = tempfile::tempdir().unwrap();
    let source = pink_noise_opus(dir.path()).await;
    let info = probe(&ffprobe(), &source).await.unwrap();
    assert_eq!(info.codec, "opus");
    assert!((info.duration_s - 10.0).abs() <= 0.2, "{info:?}");
    assert_eq!(info.channels, Some(2));

    for case in CASES {
        let resolved = profile(case.id).unwrap().resolve("opus");
        let output = dir.path().join(format!("saida_{}.{}", case.id, case.ext));
        let progress = Mutex::new(Vec::<u8>::new());
        convert(
            &ffmpeg(),
            &source,
            &output,
            &resolved,
            info.duration_s,
            &CancellationToken::new(),
            &|p| progress.lock().unwrap().push(p),
        )
        .await
        .unwrap_or_else(|e| panic!("{}: {e}", case.id));

        let out = probe(&ffprobe(), &output).await.unwrap();
        assert_eq!(out.codec, case.codec, "{}: {out:?}", case.id);
        let kbps = out
            .bitrate_kbps
            .unwrap_or_else(|| panic!("{}: sem bitrate", case.id));
        assert!(
            (case.bitrate.0..=case.bitrate.1).contains(&kbps),
            "{}: bitrate {kbps} fora de {:?}",
            case.id,
            case.bitrate
        );
        assert!(
            (out.duration_s - 10.0).abs() <= 0.2,
            "{}: duração {}",
            case.id,
            out.duration_s
        );
        assert_eq!(out.channels, Some(2), "{}", case.id);

        let progress = progress.into_inner().unwrap();
        assert_eq!(progress.last(), Some(&100), "{}: {progress:?}", case.id);
        assert!(
            progress.windows(2).all(|w| w[0] < w[1]),
            "estritamente crescente: {progress:?}"
        );
    }
}

#[tokio::test]
async fn conversao_de_arquivo_inexistente_e_erro_ffmpeg() {
    let dir = tempfile::tempdir().unwrap();
    let resolved = profile("mp3_v0").unwrap().resolve("opus");
    let err = convert(
        &ffmpeg(),
        &dir.path().join("nao-existe.opus"),
        &dir.path().join("x.mp3"),
        &resolved,
        10.0,
        &CancellationToken::new(),
        &|_| {},
    )
    .await
    .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Ffmpeg);
    assert!(!err.stderr_tail.is_empty());
}

#[tokio::test]
async fn cancelar_a_conversao_para_o_ffmpeg() {
    let dir = tempfile::tempdir().unwrap();
    let source = pink_noise_opus(dir.path()).await;
    let resolved = profile("flac").unwrap().resolve("opus");
    let cancel = CancellationToken::new();
    cancel.cancel();
    let err = convert(
        &ffmpeg(),
        &source,
        &dir.path().join("x.flac"),
        &resolved,
        10.0,
        &cancel,
        &|_| {},
    )
    .await
    .unwrap_err();
    assert_eq!(err.kind, ErrorKind::Cancelled);
}

#[tokio::test]
async fn probe_de_arquivo_que_nao_e_audio_falha() {
    let dir = tempfile::tempdir().unwrap();
    let lixo = dir.path().join("lixo.opus");
    std::fs::write(&lixo, b"isto nao e audio").unwrap();
    let err = probe(&ffprobe(), &lixo).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::Ffmpeg);
}

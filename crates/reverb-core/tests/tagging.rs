//! F09 T4 — gravação e releitura nos quatro contêineres reais.

mod common;

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::Duration;

use lofty::file::TaggedFileExt;
use lofty::prelude::{ItemKey, TagExt};
use lofty::probe::Probe;
use reverb_core::loudness::Loudness;
use reverb_core::lyrics::Lyrics;
use reverb_core::metadata::MetadataFields;
use reverb_core::tagging::{read_tags, write_tags, TagCover, TrackTags};
use reverb_core::tools::run_capture;

async fn audio(dir: &Path, ext: &str) -> PathBuf {
    let file = dir.join(format!("original.{ext}"));
    let codec = match ext {
        "mp3" => "libmp3lame",
        "m4a" => "aac",
        "opus" => "libopus",
        "flac" => "flac",
        _ => unreachable!(),
    };
    let mut args = [
        "-hide_banner",
        "-nostdin",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:duration=0.5",
        "-c:a",
        codec,
    ]
    .map(String::from)
    .to_vec();
    args.push(file.to_string_lossy().into_owned());
    let out = run_capture(common::ffmpeg(), args, |_| {}, Duration::from_secs(60))
        .await
        .unwrap();
    assert!(out.success, "{}", out.stderr);
    file
}

fn tags(opus: bool) -> TrackTags {
    let mut tags = TrackTags::from_metadata(
        &MetadataFields {
            title: "Canção — 東京".into(),
            artist: Some("João & Banda".into()),
            album: Some("Álbum".into()),
            album_artist: Some("Banda".into()),
            year: Some(2026),
            track_no: Some(3),
            track_total: Some(12),
            disc_no: Some(2),
            genre: Some("Rock".into()),
            ..Default::default()
        },
        "https://www.youtube.com/watch?v=example",
    );
    tags.isrc = Some("BRABC2600001".into());
    tags.set_lyrics(&Lyrics {
        plain: Some("simples".into()),
        synced: Some("[00:01.00]<00:01.00>Canção <00:01.50>東京\n[00:03.00]fim\n".into()),
    });
    tags.set_replay_gain(
        &Loudness {
            integrated_lufs: -14.2,
            true_peak_dbfs: -1.5,
        }
        .replay_gain()
        .unwrap(),
        opus,
    );
    let mut jpeg = Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(24, 24)
        .write_to(&mut jpeg, image::ImageFormat::Jpeg)
        .unwrap();
    tags.cover = Some(TagCover {
        mime_type: "image/jpeg".into(),
        data: jpeg.into_inner(),
    });
    tags
}

async fn audio_hash(path: &Path) -> String {
    let args = vec![
        "-hide_banner".into(),
        "-nostdin".into(),
        "-i".into(),
        path.to_string_lossy().into_owned(),
        "-map".into(),
        "0:a:0".into(),
        "-f".into(),
        "hash".into(),
        "-hash".into(),
        "sha256".into(),
        "-".into(),
    ];
    let out = run_capture(common::ffmpeg(), args, |_| {}, Duration::from_secs(60))
        .await
        .unwrap();
    assert!(out.success, "{}", out.stderr);
    out.stdout
}

async fn roundtrip(ext: &str) {
    let dir = tempfile::tempdir().unwrap();
    let path = audio(dir.path(), ext).await;
    let hash = audio_hash(&path).await;
    // Campo fora do contrato do Reverb deve sobreviver à edição.
    let mut file = Probe::open(&path).unwrap().read().unwrap();
    let primary = file.primary_tag_type();
    if file.primary_tag().is_none() {
        file.insert_tag(lofty::tag::Tag::new(primary));
    }
    let tag = file.primary_tag_mut().unwrap();
    tag.insert_text(ItemKey::Composer, "Compositor preservado".into());
    tag.save_to_path(&path, lofty::config::WriteOptions::new())
        .unwrap();

    let expected = tags(ext == "opus");
    write_tags(&path, &expected).unwrap();
    assert_eq!(read_tags(&path).unwrap(), expected, "{ext}");
    let file = Probe::open(&path).unwrap().read().unwrap();
    assert_eq!(
        file.primary_tag().unwrap().get_string(ItemKey::Composer),
        Some("Compositor preservado")
    );
    let args = vec![
        "-v".into(),
        "error".into(),
        "-show_format".into(),
        "-show_streams".into(),
        "-of".into(),
        "json".into(),
        path.to_string_lossy().into_owned(),
    ];
    let out = run_capture(common::ffprobe(), args, |_| {}, Duration::from_secs(60))
        .await
        .unwrap();
    assert!(out.success, "{}", out.stderr);
    let json: serde_json::Value = serde_json::from_str(&out.stdout).unwrap();
    // ffprobe coloca os VorbisComments de Opus nas tags do stream, não nas do formato.
    let probe_tags = if ext == "opus" {
        &json["streams"][0]["tags"]
    } else {
        &json["format"]["tags"]
    };
    for (key, value) in [
        ("title", expected.title.as_str()),
        ("artist", expected.artist.as_deref().unwrap()),
        ("album", expected.album.as_deref().unwrap()),
    ] {
        let actual = probe_tags
            .as_object()
            .unwrap()
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(key))
            .unwrap()
            .1;
        assert_eq!(actual.as_str(), Some(value), "{ext}: {key}");
    }
    assert_eq!(
        audio_hash(&path).await,
        hash,
        "tags não alteram o áudio: {ext}"
    );
    let bytes = std::fs::read(&path).unwrap();
    if ext == "mp3" {
        assert_eq!(&bytes[..4], b"ID3\x04");
        assert!(bytes.windows(4).any(|s| s == b"TXXX"));
    }
    if ext == "opus" {
        assert!(bytes.windows(22).any(|s| s == b"METADATA_BLOCK_PICTURE"));
    }
    if ext == "m4a" {
        assert!(bytes.windows(4).any(|s| s == b"\xa9day"));
        assert!(bytes.windows(4).any(|s| s == b"trkn"));
    }
    write_tags(
        &path,
        &TrackTags {
            title: "Editado".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        read_tags(&path).unwrap(),
        TrackTags {
            title: "Editado".into(),
            ..Default::default()
        }
    );
    // Datas com menos de quatro algarismos precisam do mesmo round trip.
    let ancient = TrackTags {
        title: "Editado".into(),
        year: Some(1),
        ..Default::default()
    };
    write_tags(&path, &ancient).unwrap();
    assert_eq!(read_tags(&path).unwrap(), ancient);
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        1,
        "nenhum staging restante"
    );
}

#[tokio::test]
async fn t4_mp3_id3v24() {
    roundtrip("mp3").await;
}
#[tokio::test]
async fn t4_m4a_ilst() {
    roundtrip("m4a").await;
}
#[tokio::test]
async fn t4_opus_vorbis() {
    roundtrip("opus").await;
}
#[tokio::test]
async fn t4_flac_vorbis() {
    roundtrip("flac").await;
}

#[tokio::test]
async fn erro_de_capa_ou_campo_invalido_preserva_original() {
    let dir = tempfile::tempdir().unwrap();
    let path = audio(dir.path(), "mp3").await;
    let before = std::fs::read(&path).unwrap();
    let mut invalid = tags(false);
    invalid.cover.as_mut().unwrap().data = b"imagem invalida".to_vec();
    assert!(write_tags(&path, &invalid).is_err());
    invalid = tags(false);
    invalid.artist = Some("a\0b".into());
    assert!(write_tags(&path, &invalid).is_err());
    invalid = tags(false);
    invalid.r128_track_gain = Some(42);
    assert!(write_tags(&path, &invalid).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[tokio::test]
async fn somente_leitura_retorna_disk_sem_tocar_no_arquivo() {
    let dir = tempfile::tempdir().unwrap();
    let path = audio(dir.path(), "flac").await;
    let before = std::fs::read(&path).unwrap();
    let original_permissions = std::fs::metadata(&path).unwrap().permissions();
    let mut readonly = original_permissions.clone();
    readonly.set_readonly(true);
    std::fs::set_permissions(&path, readonly).unwrap();
    let result = write_tags(&path, &tags(false));
    std::fs::set_permissions(&path, original_permissions).unwrap();
    assert_eq!(result.unwrap_err().kind(), "disk");
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn arquivo_invalido_nao_e_modificado() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid.mp3");
    std::fs::write(&path, "não é áudio").unwrap();
    assert!(read_tags(&path).is_err());
    assert!(write_tags(&path, &TrackTags::default()).is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), "não é áudio");
}

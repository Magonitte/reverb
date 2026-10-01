//! T4 — modelos a partir das fixtures gravadas (FX1–FX4).

use super::*;

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/ytdlp")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn fx1_video_curto_nao_e_faixa_oficial() {
    let info = VideoInfo::from_json(&fixture("fx1-video.json")).unwrap();
    assert_eq!(info.id, "jNQXAC9IVRw");
    assert_eq!(info.title, "Me at the zoo");
    assert_eq!(info.duration, Some(19.0));
    assert_eq!(info.channel.as_deref(), Some("jawed"));
    assert_eq!(info.track, None);
    assert!(!info.is_official_track);
    assert_eq!(info.chapters.len(), 3);
    assert_eq!(info.chapters[1].title, "The cool thing");
    assert_eq!(info.chapters[1].end_time, 17.0);
    // O melhor áudio por bitrate é o AAC 140 (~130 kbps); o Opus 251 tem ~106 kbps.
    assert_eq!(info.best_audio_abr.map(|a| a.round()), Some(130.0));
    assert!(info.audio_formats.iter().any(|f| f.format_id == "251"));
}

#[test]
fn fx2_faixa_oficial_do_youtube_music() {
    let info = VideoInfo::from_json(&fixture("fx2-music.json")).unwrap();
    assert!(info.is_official_track);
    assert_eq!(info.track.as_deref(), Some("Never Gonna Give You Up"));
    assert_eq!(info.artist.as_deref(), Some("Rick Astley"));
    assert_eq!(info.artists, ["Rick Astley"]);
    assert_eq!(info.album.as_deref(), Some("Whenever You Need Somebody"));
    assert_eq!(info.release_year, Some(1987));
    assert_eq!(info.release_date.as_deref(), Some("19871112"));
    assert_eq!(info.categories, ["Music"]);
    assert_eq!(info.duration, Some(214.0));
    assert!(info.chapters.is_empty(), "chapters null vira lista vazia");
    assert_eq!(info.extractor_key.as_deref(), Some("Youtube"));
}

#[test]
fn fx3_clipe_nao_e_faixa_oficial() {
    let info = VideoInfo::from_json(&fixture("fx3-clip.json")).unwrap();
    assert_eq!(info.id, "dQw4w9WgXcQ");
    assert!(!info.is_official_track);
    assert_eq!(info.track, None);
    assert_eq!(info.album, None);
    assert_eq!(info.duration, Some(213.0));
    assert_eq!(info.channel.as_deref(), Some("Rick Astley"));
}

#[test]
fn audio_formats_exclui_storyboards_video_e_formatos_sem_codec() {
    let info = VideoInfo::from_json(&fixture("fx2-music.json")).unwrap();
    let ids: Vec<&str> = info
        .audio_formats
        .iter()
        .map(|f| f.format_id.as_str())
        .collect();
    for esperado in ["139", "249", "250", "140", "251"] {
        assert!(ids.contains(&esperado), "{esperado} em {ids:?}");
    }
    for proibido in ["sb0", "sb1", "sb2", "sb3", "233", "234", "602"] {
        assert!(
            !ids.contains(&proibido),
            "{proibido} não é áudio utilizável"
        );
    }
    assert!(info
        .audio_formats
        .iter()
        .all(|f| f.acodec != "none" && f.ext != "mhtml"));
    assert_eq!(info.best_audio_abr.map(|a| a.round()), Some(130.0));
}

#[test]
fn fx4_album_tem_10_entradas_com_duracao() {
    let info = CollectionInfo::from_json(&fixture("fx4-album.json")).unwrap();
    assert_eq!(
        info.title.as_deref(),
        Some("Album - Whenever You Need Somebody")
    );
    assert_eq!(info.entries.len(), 10);
    assert!(info
        .entries
        .iter()
        .all(|e| e.duration.is_some() && e.title.is_some()));
    let first = &info.entries[0];
    assert_eq!(
        (first.id.as_str(), first.duration),
        ("lYBUbBu4W08", Some(214.0))
    );
    let second = &info.entries[1];
    assert_eq!(second.id, "raBobo3GZYA");
    assert_eq!(second.title.as_deref(), Some("Whenever You Need Somebody"));
    assert_eq!(second.duration, Some(234.0));
    assert_eq!(info.entries[2].id, "i_Q88T1HI_w");
    assert_eq!(info.entries[3].id, "dc7UCha20yw");
}

#[test]
fn colecao_descarta_entradas_nulas_ou_sem_id() {
    let json = r#"{"id":"PL1","title":"x","uploader":"Canal","entries":[null,{"title":"sem id"},{"id":"abc","title":"ok","duration":null}]}"#;
    let info = CollectionInfo::from_json(json).unwrap();
    assert_eq!(info.entries.len(), 1);
    assert_eq!(info.entries[0].id, "abc");
    assert_eq!(info.channel.as_deref(), Some("Canal"), "cai para uploader");
}

#[test]
fn resultados_de_busca_do_ytmusic_nao_trazem_duracao() {
    let json = r#"{"entries":[
        {"id":"lYBUbBu4W08","title":"Never Gonna Give You Up","url":"https://www.youtube.com/watch?v=lYBUbBu4W08","duration":null,"channel":null},
        {"id":"wxu4rvTr1L4","title":"Never Gonna Give You Up (En Directo)","url":"https://www.youtube.com/watch?v=wxu4rvTr1L4"},
        null,
        {"title":"sem id"}
    ]}"#;
    let results = SearchResult::list_from_json(json).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, "lYBUbBu4W08");
    assert_eq!(results[0].duration, None);
    assert_eq!(results[1].title, "Never Gonna Give You Up (En Directo)");
}

#[test]
fn json_invalido_e_erro() {
    assert!(VideoInfo::from_json("não é json").is_err());
    assert!(VideoInfo::from_json("{}").is_err(), "id é obrigatório");
    assert!(CollectionInfo::from_json("[]").is_err());
}

#[test]
fn track_sem_artista_nao_e_oficial() {
    let json = r#"{"id":"abc","title":"t","track":"Faixa"}"#;
    assert!(!VideoInfo::from_json(json).unwrap().is_official_track);
    let json = r#"{"id":"abc","title":"t","track":"Faixa","artists":["A"]}"#;
    assert!(VideoInfo::from_json(json).unwrap().is_official_track);
    let json = r#"{"id":"abc","title":"t","track":"","artist":"A"}"#;
    assert!(!VideoInfo::from_json(json).unwrap().is_official_track);
}

#[test]
fn serializa_em_camel_case() {
    let info = VideoInfo::from_json(&fixture("fx2-music.json")).unwrap();
    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["isOfficialTrack"], true);
    assert_eq!(json["releaseYear"], 1987);
    assert!(json["audioFormats"].is_array());
}

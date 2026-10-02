use super::*;
use crate::metadata::provider::Candidate;
use crate::metadata::{Bucket, MetadataFields, MetadataResult};
use crate::queue::{JobOptions, JobStage, JobStatus};
use crate::Db;

fn job() -> Job {
    Job {
        id: "job-1".into(),
        kind: "single".into(),
        provider: "youtube".into(),
        source_url: "https://www.youtube.com/watch?v=official".into(),
        source_id: Some("official".into()),
        title: Some("Título antigo".into()),
        artist: Some("Artista antigo".into()),
        thumbnail: None,
        duration_s: Some(200.0),
        profile_id: "opus_160".into(),
        options: JobOptions::default(),
        metadata_override: None,
        confidence: Some(0.5),
        metadata_result: Some(MetadataResult {
            fields: MetadataFields {
                title: "Identificado".into(),
                mb_recording_id: Some("mb-id".into()),
                ..Default::default()
            },
            confidence: 0.85,
            source: "deezer".into(),
            bucket: Bucket::Review,
            candidates: vec![ScoredCandidate {
                score: 0.85,
                candidate: Candidate::new("deezer", "123", "Identificado"),
            }],
            content_type: ContentType::Music,
            isrc: Some("gb-arl-93-00135".into()),
            official: None,
        }),
        warnings: vec![],
        playlist_ctx: None,
        sync_id: None,
        status: JobStatus::Running,
        stage: JobStage::Moving,
        progress: 0.0,
        overall_progress: 0.0,
        speed_bps: None,
        eta_s: None,
        error_kind: None,
        error_message: None,
        attempts: 1,
        output_path: None,
        library_id: None,
        position: 1,
        created_at: 1,
        updated_at: 1,
        finished_at: None,
    }
}

fn file() -> DownloadedFile {
    DownloadedFile {
        file_path: "library/Artista/Álbum/Música.opus".into(),
        tags: TrackTags {
            title: "Música — 東京".into(),
            artist: Some("Artista final".into()),
            album: Some("Álbum final".into()),
            album_artist: Some("Álbum artista".into()),
            track_no: Some(3),
            track_total: Some(12),
            disc_no: Some(2),
            year: Some(1987),
            genre: Some("Pop".into()),
            lyrics: Some("[00:01.00]<00:01.00>Música\n".into()),
            ..Default::default()
        },
        probe: Some(ProbeInfo {
            codec: "opus".into(),
            duration_s: 213.5,
            bitrate_kbps: Some(160),
            sample_rate: Some(48000),
            channels: Some(2),
        }),
        source_abr_kbps: Some(125.5),
        content_type: ContentType::Music,
        has_synced_lyrics: true,
        cover_source: Some("deezer".into()),
        replaygain_db: Some(-3.8),
    }
}

#[test]
fn insere_campos_finais_identificacao_e_fts_sem_acento() {
    let db = Db::open_in_memory().unwrap();
    db.call_blocking(|conn| {
        let job = job();
        let file = file();
        let before = crate::queue::repo::now();
        let id = insert_from_job(conn, &job, &file)?;
        let item = get(conn, id)?.unwrap();
        assert_eq!(item.id, id);
        assert_eq!(item.file_path, file.file_path);
        assert_eq!(item.title, file.tags.title);
        assert_eq!(item.artist, file.tags.artist);
        assert_eq!(item.album, file.tags.album);
        assert_eq!(item.album_artist, file.tags.album_artist);
        assert_eq!(
            (item.track_no, item.track_total, item.disc_no, item.year),
            (Some(3), Some(12), Some(2), Some(1987))
        );
        assert_eq!(item.genre.as_deref(), Some("Pop"));
        assert_eq!(
            (item.duration_s, item.bitrate_kbps, item.source_abr_kbps),
            (Some(213.5), Some(160.0), Some(125.5))
        );
        assert_eq!(item.codec.as_deref(), Some("opus"));
        assert_eq!(item.provider.as_deref(), Some("youtube"));
        assert_eq!(item.source_id, job.source_id);
        assert_eq!(item.source_url.as_deref(), Some(job.source_url.as_str()));
        assert_eq!(item.profile_id.as_deref(), Some("opus_160"));
        assert_eq!(item.isrc.as_deref(), Some("GBARL9300135"));
        assert_eq!(item.metadata_source.as_deref(), Some("deezer"));
        assert_eq!(item.confidence, Some(0.85));
        assert!(item.needs_review);
        assert_eq!(
            item.review_candidates.as_deref(),
            Some(job.metadata_result.as_ref().unwrap().candidates.as_slice())
        );
        assert!(item.has_lyrics && item.has_synced_lyrics);
        assert_eq!(item.cover_source.as_deref(), Some("deezer"));
        assert_eq!(item.mb_recording_id.as_deref(), Some("mb-id"));
        assert_eq!(item.replaygain_db, Some(-3.8));
        assert_eq!(item.origin, "download");
        assert_eq!(item.content_type, ContentType::Music);
        assert!(!item.missing);
        assert!(item.acoustid_id.is_none() && item.lossless_verdict.is_none());
        assert!(item.added_at >= before && item.added_at <= crate::queue::repo::now());
        assert_eq!(item.added_at, item.updated_at);
        for query in ["musica", "artista", "album", "東京"] {
            let found: i64 = conn.query_row(
                "SELECT rowid FROM library_fts WHERE library_fts MATCH ?1",
                [query],
                |r| r.get(0),
            )?;
            assert_eq!(found, id);
        }
        Ok(())
    })
    .unwrap();
}

#[test]
fn sem_metadados_other_preserva_nulos_e_duracao_da_origem() {
    let db = Db::open_in_memory().unwrap();
    db.call_blocking(|conn| {
        let mut job = job();
        job.metadata_result = None;
        job.source_id = None;
        let mut file = file();
        file.probe = None;
        file.content_type = ContentType::Other;
        file.tags = TrackTags {
            title: "Me at the zoo".into(),
            ..Default::default()
        };
        file.has_synced_lyrics = false;
        file.source_abr_kbps = None;
        file.cover_source = None;
        file.replaygain_db = None;
        let item = get(conn, insert_from_job(conn, &job, &file)?)?.unwrap();
        assert_eq!(item.content_type, ContentType::Other);
        assert_eq!(item.duration_s, job.duration_s);
        assert!(item.artist.is_none() && item.album.is_none() && item.isrc.is_none());
        assert!(
            item.confidence.is_none()
                && item.metadata_source.is_none()
                && item.review_candidates.is_none()
        );
        assert!(item.codec.is_none() && item.bitrate_kbps.is_none() && item.source_id.is_none());
        assert!(!item.has_lyrics && !item.has_synced_lyrics && !item.needs_review);
        Ok(())
    })
    .unwrap();
}

#[test]
fn letras_simples_e_metadados_aplicados_nao_entram_em_revisao() {
    let db = Db::open_in_memory().unwrap();
    db.call_blocking(|conn| {
        let mut job = job();
        job.metadata_result.as_mut().unwrap().bucket = Bucket::Auto;
        let mut file = file();
        file.has_synced_lyrics = false;
        file.tags.lyrics = Some("Letra simples".into());
        let item = get(conn, insert_from_job(conn, &job, &file)?)?.unwrap();
        assert!(item.has_lyrics && !item.has_synced_lyrics);
        assert!(!item.needs_review && item.review_candidates.is_none());
        Ok(())
    })
    .unwrap();
}

#[test]
fn fonte_e_perfil_sao_parte_da_identidade_e_busca_tem_ordem_estavel() {
    let db = Db::open_in_memory().unwrap();
    db.call_blocking(|conn| {
        let mut job = job();
        let mut file = file();
        let first = insert_from_job(conn, &job, &file)?;
        file.file_path = "second.opus".into();
        let second = insert_from_job(conn, &job, &file)?;
        job.profile_id = "mp3_v0".into();
        file.file_path = "third.mp3".into();
        let third = insert_from_job(conn, &job, &file)?;
        job.provider = "soundcloud".into();
        file.file_path = "fourth.mp3".into();
        let fourth = insert_from_job(conn, &job, &file)?;
        assert_eq!(
            find_by_source(conn, "youtube", "official", "opus_160")?
                .unwrap()
                .id,
            first
        );
        assert_eq!(
            find_by_source(conn, "youtube", "official", "mp3_v0")?
                .unwrap()
                .id,
            third
        );
        assert_eq!(
            find_by_source(conn, "soundcloud", "official", "mp3_v0")?
                .unwrap()
                .id,
            fourth
        );
        assert!(find_by_source(conn, "youtube", "missing", "opus_160")?.is_none());
        assert!(find_by_source(conn, "youtube", "official", "flac")?.is_none());
        let hits = find_by_isrc(conn, "gb arl 93 00135")?;
        assert_eq!(
            hits.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![first, second, third, fourth]
        );
        assert!(find_by_isrc(conn, "GBARL0600786")?.is_empty());
        assert!(find_by_isrc(conn, "invalid")?.is_empty());
        assert!(get(conn, 9999)?.is_none());
        Ok(())
    })
    .unwrap();
}

#[test]
fn isrc_final_completa_identificacao_ausente_e_invalido_e_ignorado() {
    let db = Db::open_in_memory().unwrap();
    db.call_blocking(|conn| {
        let mut job = job();
        job.metadata_result.as_mut().unwrap().isrc = None;
        let mut file = file();
        file.tags.isrc = Some("gb-arl-06-00786".into());
        let id = insert_from_job(conn, &job, &file)?;
        assert_eq!(
            get(conn, id)?.unwrap().isrc.as_deref(),
            Some("GBARL0600786")
        );
        file.file_path = "invalid.opus".into();
        file.tags.isrc = Some("invalid".into());
        let id = insert_from_job(conn, &job, &file)?;
        assert!(get(conn, id)?.unwrap().isrc.is_none());
        Ok(())
    })
    .unwrap();
}

#[test]
fn caminho_duplicado_nao_altera_registro_ou_fts() {
    let db = Db::open_in_memory().unwrap();
    db.call_blocking(|conn| {
        let mut file = file();
        let job = job();
        let id = insert_from_job(conn, &job, &file)?;
        let original = get(conn, id)?;
        file.tags.title = "Sobrescrito".into();
        assert!(insert_from_job(conn, &job, &file).is_err());
        assert_eq!(get(conn, id)?, original);
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM library_fts WHERE library_fts MATCH 'sobrescrito'",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(count, 0);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM library", [], |r| r.get::<_, i64>(0))?,
            1
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn dados_invalidos_nao_criam_registros() {
    let db = Db::open_in_memory().unwrap();
    db.call_blocking(|conn| {
        let job = job();
        for n in 0..7 {
            let mut file = file();
            match n {
                0 => file.file_path.clear(),
                1 => file.tags.title = " ".into(),
                2 => file.tags.lyrics = None,
                3 => file.probe.as_mut().unwrap().duration_s = f64::NAN,
                4 => file.source_abr_kbps = Some(-1.0),
                5 => file.replaygain_db = Some(f64::INFINITY),
                _ => file.file_path = "a\0b".into(),
            }
            assert!(insert_from_job(conn, &job, &file).is_err(), "caso {n}");
        }
        let mut job = job.clone();
        job.metadata_result.as_mut().unwrap().candidates[0].score = f64::NAN;
        assert!(insert_from_job(conn, &job, &file()).is_err());
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM library", [], |r| r.get::<_, i64>(0))?,
            0
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn persiste_ao_reabrir_e_pode_participar_de_transacao_do_chamador() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.sqlite");
    let db = Db::open(&path).unwrap();
    let id = db
        .call_blocking(|conn| insert_from_job(conn, &job(), &file()))
        .unwrap();
    drop(db);
    let db = Db::open(&path).unwrap();
    db.call_blocking(|conn| {
        assert!(get(conn, id)?.is_some());
        let tx = conn.transaction()?;
        let mut file = file();
        file.file_path = "rolled-back.opus".into();
        file.tags.title = "Rollback".into();
        let rolled_back = insert_from_job(&tx, &job(), &file)?;
        tx.rollback()?;
        assert!(get(conn, rolled_back)?.is_none());
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM library_fts WHERE library_fts MATCH 'rollback'",
                [],
                |r| r.get::<_, i64>(0)
            )?,
            0
        );
        Ok(())
    })
    .unwrap();
}

#[test]
fn t9_fila_detecta_registro_criado_pelo_repositorio_sem_job_na_fila() {
    let db = Db::open_in_memory().unwrap();
    db.call_blocking(|conn| {
        let job = job();
        insert_from_job(conn, &job, &file())?;
        let hits = crate::queue::repo::find_duplicates(
            conn,
            "youtube",
            &["official".into(), "unknown".into()],
            "opus_160",
        )?;
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].found_in, "library");
        assert!(hits[0].job_id.is_none());
        assert!(crate::queue::repo::find_duplicates(
            conn,
            "youtube",
            &["official".into()],
            "mp3_v0"
        )?
        .is_empty());
        assert!(crate::queue::repo::find_duplicates(
            conn,
            "soundcloud",
            &["official".into()],
            "opus_160"
        )?
        .is_empty());
        Ok(())
    })
    .unwrap();
}

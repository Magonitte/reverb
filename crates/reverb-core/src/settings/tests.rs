use std::sync::Arc;

use serde_json::json;
use tempfile::tempdir;

use super::*;
use crate::events::MemorySink;

fn patch(value: serde_json::Value) -> SettingsPatch {
    serde_json::from_value(value).expect("patch válido")
}

async fn service() -> (SettingsService, Arc<MemorySink>) {
    let sink = Arc::new(MemorySink::new());
    let svc = SettingsService::new(Db::open_in_memory().unwrap(), sink.clone())
        .await
        .unwrap();
    (svc, sink)
}

#[test]
fn t5_padroes_batem_com_a_tabela_6() {
    let s = Settings::default();
    assert_eq!(s.output_dir, "");
    assert_eq!(
        s.file_template,
        "{albumartist}/{album}/{track:02} - {title}"
    );
    assert!(s.auto_organize);
    assert_eq!(s.default_profile, "original");
    assert_eq!(s.parallelism, 2);
    assert_eq!(s.speed_limit_mbps, 0.0);
    assert_eq!(s.queue_limit, 500);
    assert_eq!(s.max_attempts, 3);
    assert!(s.fetch_metadata);
    assert!(s.extract_title_from_video);
    assert!(s.prefer_official_audio);
    assert_eq!(s.confidence_auto_apply, 0.85);
    assert_eq!(s.confidence_review, 0.60);
    assert!(!s.offline_mode);
    assert!(s.fetch_artwork);
    assert!(s.write_folder_cover);
    assert!(s.fetch_lyrics);
    assert!(s.write_lrc_file);
    assert!(s.normalize_volume);
    assert!(!s.sponsorblock_remove);
    assert_eq!(s.sponsorblock_categories, vec!["music_offtopic"]);
    assert_eq!(s.split_chapters, SplitChapters::Ask);
    assert!(!s.trim_silence);
    assert_eq!(s.playlist_pacing_seconds, 3);
    assert!(s.watch_library);
    assert_eq!(s.theme, Theme::Dark);
    assert_eq!(s.language, Language::PtBr);
    assert_eq!(s.transparency, Transparency::Auto);
    assert!(!s.launch_at_startup);
    assert!(s.start_minimized);
    assert!(s.minimize_to_tray);
    assert!(s.close_to_tray);
    assert!(s.completion_notifications);
    assert!(!s.clipboard_watch);
    assert_eq!(s.global_shortcut, "");
    assert!(s.weekly_self_test);
    assert!(!s.onboarding_completed);
    assert_eq!(s.cookies_source, CookiesSource::None);
    assert_eq!(s.cookies_file, "");
    assert_eq!(s.ytdlp_channel, YtdlpChannel::Stable);
    assert_eq!(s.js_runtime, JsRuntime::Auto);
    assert!(s.auto_update_tools);
    assert_eq!(s.pot_provider, PotProvider::Auto);
    assert_eq!(s.quality_target_kbps, 0);
    assert!(!s.auto_upgrade);
    assert_eq!(s.artist_check_interval_hours, 24);
    assert!(s.auto_check_app_updates);
    assert!(s.verify_lossless_on_import);
    for secret in s.secret_values() {
        assert_eq!(secret, "");
    }
    assert!(validate(&s).is_ok());
}

#[test]
fn chaves_serializam_em_camel_case_com_enums_em_texto() {
    let json = serde_json::to_value(Settings::default()).unwrap();
    assert_eq!(json["speedLimitMbps"], 0.0);
    assert_eq!(json["language"], "pt-BR");
    assert_eq!(json["jsRuntime"], "auto");
    assert!(json.get("acoustidKey").is_some());
}

#[test]
fn view_tem_as_mesmas_chaves_menos_os_segredos() {
    let settings = Settings::default();
    let full = serde_json::to_value(&settings).unwrap();
    let view = serde_json::to_value(settings.view()).unwrap();
    let secrets = [
        "acoustidKey",
        "spotifyClientId",
        "spotifyClientSecret",
        "discogsToken",
        "jamendoClientId",
    ];
    for key in full.as_object().unwrap().keys() {
        assert_eq!(
            view.get(key).is_some(),
            !secrets.contains(&key.as_str()),
            "{key}"
        );
    }
    assert!(view.get("secretsStatus").is_some());
}

#[tokio::test]
async fn t6_parallelism_fora_de_1_a_4_e_erro() {
    let (svc, _) = service().await;
    for bad in [0, 5] {
        let err = svc
            .update(patch(json!({"parallelism": bad})))
            .await
            .unwrap_err();
        assert_eq!(err.kind(), "invalid");
        assert_eq!(err.i18n_key(), Some("errors.settings.parallelism"));
    }
    for good in [1, 4] {
        let updated = svc
            .update(patch(json!({"parallelism": good})))
            .await
            .unwrap();
        assert_eq!(updated.parallelism, good);
    }
}

#[tokio::test]
async fn t6_confianca_auto_deve_superar_a_de_revisao() {
    let (svc, _) = service().await;
    let err = svc
        .update(patch(json!({"confidenceAutoApply": 0.6})))
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "invalid");
    let err = svc
        .update(patch(json!({"confidenceReview": 0.9})))
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "invalid");
    // Alterar os dois juntos para um par coerente é aceito.
    let ok = svc
        .update(patch(
            json!({"confidenceReview": 0.5, "confidenceAutoApply": 0.7}),
        ))
        .await
        .unwrap();
    assert_eq!(ok.confidence_review, 0.5);
}

#[tokio::test]
async fn t6_output_dir_relativo_e_erro_e_absoluto_e_criado() {
    let (svc, _) = service().await;
    assert!(svc
        .update(patch(json!({"outputDir": "musicas/relativa"})))
        .await
        .is_err());

    let tmp = tempdir().unwrap();
    let target = tmp.path().join("a").join("b");
    let updated = svc
        .update(patch(json!({"outputDir": target.to_string_lossy()})))
        .await
        .unwrap();
    assert_eq!(updated.output_dir, target.to_string_lossy());
    assert!(target.is_dir());
}

#[test]
fn t6_chave_desconhecida_no_patch_e_erro() {
    let result: Result<SettingsPatch, _> = serde_json::from_value(json!({"naoExiste": 1}));
    assert!(result.is_err());
}

#[tokio::test]
async fn t6_outras_validacoes_do_plano() {
    let (svc, _) = service().await;
    let bad = [
        json!({"defaultProfile": "mp3_999"}),
        json!({"fileTemplate": "{titulo}"}),
        json!({"fileTemplate": "{title"}),
        json!({"fileTemplate": "{title:02}"}),
        json!({"fileTemplate": ""}),
        json!({"speedLimitMbps": 1001.0}),
        json!({"speedLimitMbps": -1.0}),
        json!({"queueLimit": 9}),
        json!({"maxAttempts": 11}),
        json!({"playlistPacingSeconds": 61}),
        json!({"sponsorblockCategories": ["nao_existe"]}),
        json!({"qualityTargetKbps": 128}),
        json!({"artistCheckIntervalHours": 5}),
        json!({"globalShortcut": "D"}),
        json!({"globalShortcut": "Ctrl+"}),
        json!({"cookiesSource": "file", "cookiesFile": "/nao/existe.txt"}),
    ];
    for value in bad {
        assert!(
            svc.update(patch(value.clone())).await.is_err(),
            "deveria falhar: {value}"
        );
    }
    let good = [
        json!({"fileTemplate": "{artist} - {title}"}),
        json!({"fileTemplate": "{albumartist}/{album}/{disc}-{track:02} {title}"}),
        json!({"defaultProfile": "flac"}),
        json!({"qualityTargetKbps": 256}),
        json!({"globalShortcut": "Ctrl+Shift+D"}),
        json!({"globalShortcut": "Alt+F10"}),
        json!({"globalShortcut": ""}),
        json!({"theme": "light", "language": "en", "transparency": "reduced"}),
        json!({"jsRuntime": "system-node", "potProvider": "off", "ytdlpChannel": "nightly"}),
        json!({"sponsorblockCategories": ["sponsor", "intro"]}),
    ];
    for value in good {
        svc.update(patch(value.clone()))
            .await
            .unwrap_or_else(|e| panic!("deveria passar: {value}: {e}"));
    }
}

#[tokio::test]
async fn t6_valor_de_enum_invalido_nao_desserializa() {
    let result: Result<SettingsPatch, _> = serde_json::from_value(json!({"theme": "neon"}));
    assert!(result.is_err());
}

#[tokio::test]
async fn t6_cookies_file_existente_e_aceito() {
    let (svc, _) = service().await;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("cookies.txt");
    std::fs::write(&file, "# Netscape HTTP Cookie File\n").unwrap();
    let updated = svc
        .update(patch(
            json!({"cookiesSource": "file", "cookiesFile": file.to_string_lossy()}),
        ))
        .await
        .unwrap();
    assert_eq!(updated.cookies_source, CookiesSource::File);
    assert!(
        !crate::logging::redact(&format!("cookies at {}", file.display()))
            .contains(&file.to_string_lossy().to_string())
    );
}

#[tokio::test]
async fn t7_valores_persistem_apos_reabrir_o_banco() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("reverb.db");
    {
        let db = Db::open(&path).unwrap();
        let svc = SettingsService::new(db, Arc::new(MemorySink::new()))
            .await
            .unwrap();
        svc.update(patch(json!({
            "parallelism": 4, "theme": "light", "sponsorblockCategories": ["intro"],
            "acoustidKey": "chave-persistida-t7",
        })))
        .await
        .unwrap();
    }
    let db = Db::open(&path).unwrap();
    let svc = SettingsService::new(db, Arc::new(MemorySink::new()))
        .await
        .unwrap();
    let s = svc.get();
    assert_eq!(s.parallelism, 4);
    assert_eq!(s.theme, Theme::Light);
    assert_eq!(s.sponsorblock_categories, vec!["intro"]);
    assert_eq!(s.acoustid_key, "chave-persistida-t7");
    assert_eq!(s.queue_limit, 500); // não alterado = padrão
}

#[tokio::test]
async fn so_chaves_alteradas_sao_gravadas() {
    let sink = Arc::new(MemorySink::new());
    let db = Db::open_in_memory().unwrap();
    let svc = SettingsService::new(db.clone(), sink).await.unwrap();
    svc.update(patch(json!({"parallelism": 3}))).await.unwrap();
    let keys: Vec<String> = db
        .call(|c| {
            let mut stmt = c.prepare("SELECT key FROM settings ORDER BY key")?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
        .await
        .unwrap();
    assert_eq!(keys, vec!["parallelism"]);
}

#[tokio::test]
async fn valor_gravado_invalido_e_ignorado_na_leitura() {
    let db = Db::open_in_memory().unwrap();
    db.call(|c| {
        c.execute("INSERT INTO settings VALUES ('parallelism', '\"abc\"')", [])?;
        c.execute("INSERT INTO settings VALUES ('chaveAntiga', '1')", [])?;
        c.execute("INSERT INTO settings VALUES ('theme', '\"light\"')", [])?;
        Ok(())
    })
    .await
    .unwrap();
    let svc = SettingsService::new(db, Arc::new(MemorySink::new()))
        .await
        .unwrap();
    assert_eq!(svc.get().parallelism, 2);
    assert_eq!(svc.get().theme, Theme::Light);
}

#[tokio::test]
async fn t8_view_e_evento_nao_vazam_segredos() {
    let (svc, sink) = service().await;
    svc.update(patch(json!({
        "acoustidKey": "SEGREDO-ACOUSTID-123",
        "spotifyClientId": "SPOTIFY-ID-456",
        "spotifyClientSecret": "SPOTIFY-SECRET-789",
        "discogsToken": "DISCOGS-TOKEN-000",
    })))
    .await
    .unwrap();

    let view = serde_json::to_string(&svc.view()).unwrap();
    let event = serde_json::to_string(&sink.named(EVENT_CHANGED).last().unwrap()).unwrap();
    for text in [&view, &event] {
        for secret in [
            "SEGREDO-ACOUSTID-123",
            "SPOTIFY-ID-456",
            "SPOTIFY-SECRET-789",
            "DISCOGS-TOKEN-000",
        ] {
            assert!(!text.contains(secret), "vazou {secret}: {text}");
        }
    }
    let status = svc.view().secrets_status;
    assert!(status.acoustid && status.spotify && status.discogs);
    assert!(!status.jamendo);
}

#[tokio::test]
async fn t8_spotify_exige_id_e_secret_e_limpar_funciona() {
    let (svc, _) = service().await;
    svc.update(patch(json!({"spotifyClientId": "so-o-id"})))
        .await
        .unwrap();
    assert!(!svc.view().secrets_status.spotify);
    svc.update(patch(json!({"spotifyClientSecret": "e-o-secret"})))
        .await
        .unwrap();
    assert!(svc.view().secrets_status.spotify);

    svc.update(patch(json!({"spotifyClientSecret": ""})))
        .await
        .unwrap();
    assert!(!svc.view().secrets_status.spotify);
    assert_eq!(svc.get().spotify_client_secret, "");
}

#[tokio::test]
async fn t8_reset_mantem_segredos_e_restaura_o_resto() {
    let (svc, sink) = service().await;
    svc.update(patch(
        json!({"parallelism": 4, "theme": "light", "jamendoClientId": "JAMENDO-ID-1"}),
    ))
    .await
    .unwrap();
    let after = svc.reset().await.unwrap();
    assert_eq!(after.parallelism, 2);
    assert_eq!(after.theme, Theme::Dark);
    assert_eq!(after.jamendo_client_id, "JAMENDO-ID-1");
    assert!(svc.view().secrets_status.jamendo);
    assert_eq!(sink.named(EVENT_CHANGED).len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn t9_cinquenta_updates_paralelos_sem_database_is_locked() {
    let dir = tempdir().unwrap();
    let db = Db::open(&dir.path().join("reverb.db")).unwrap();
    let svc = Arc::new(
        SettingsService::new(db, Arc::new(MemorySink::new()))
            .await
            .unwrap(),
    );

    let mut handles = Vec::new();
    for i in 0..50u32 {
        let svc = Arc::clone(&svc);
        handles.push(tokio::spawn(async move {
            svc.update(patch(
                json!({"parallelism": (i % 4) + 1, "queueLimit": 10 + i}),
            ))
            .await
        }));
    }
    for handle in handles {
        handle.await.unwrap().expect("update sem erro");
    }
    let s = svc.get();
    assert!((1..=4).contains(&s.parallelism));
    assert!((10..60).contains(&s.queue_limit));
}

#[tokio::test]
async fn t10_evento_changed_traz_o_novo_estado() {
    let (svc, sink) = service().await;
    svc.update(patch(json!({"parallelism": 3, "language": "en"})))
        .await
        .unwrap();

    let events = sink.named(EVENT_CHANGED);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["parallelism"], 3);
    assert_eq!(events[0]["language"], "en");
    assert!(events[0]["secretsStatus"].is_object());
}

#[tokio::test]
async fn update_invalido_nao_emite_evento_nem_altera_o_estado() {
    let (svc, sink) = service().await;
    assert!(svc.update(patch(json!({"parallelism": 9}))).await.is_err());
    assert!(sink.named(EVENT_CHANGED).is_empty());
    assert_eq!(svc.get().parallelism, 2);
}

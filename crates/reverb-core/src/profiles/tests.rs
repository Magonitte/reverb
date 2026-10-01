use super::*;

#[test]
fn tabela_tem_os_seis_perfis_da_arquitetura() {
    let ids: Vec<_> = PROFILES.iter().map(|p| p.id).collect();
    assert_eq!(
        ids,
        ["original", "mp3_v0", "mp3_320", "aac_256", "opus_96", "flac"]
    );
    assert!(PROFILES
        .iter()
        .skip(1)
        .all(|p| p.reencodes && !p.args.is_empty()));
}

#[test]
fn original_mantem_extensoes_conhecidas() {
    for ext in ["opus", "m4a", "mp3", "ogg", "flac", "wav", "OPUS"] {
        let resolved = profile_for_source(ext);
        assert_eq!(resolved.ext, ext.to_ascii_lowercase());
        assert!(!resolved.needs_conversion() && !resolved.fallback, "{ext}");
    }
}

#[test]
fn original_com_fonte_exotica_vira_opus_160k() {
    let resolved = profile_for_source("webm");
    assert!(resolved.fallback && resolved.reencodes);
    assert_eq!(resolved.ext, "opus");
    assert_eq!(resolved.args.join(" "), "-c:a libopus -b:a 160k");
}

#[test]
fn perfil_explicito_ignora_a_fonte() {
    let resolved = profile("mp3_320").unwrap().resolve("opus");
    assert_eq!(resolved.ext, "mp3");
    assert_eq!(resolved.args.join(" "), "-c:a libmp3lame -b:a 320k");
    assert!(profile("inexistente").is_none());
}

#[test]
fn ids_batem_com_a_lista_validada_nas_configuracoes() {
    let ids: Vec<&str> = PROFILES.iter().map(|p| p.id).collect();
    assert_eq!(ids, crate::settings::PROFILE_IDS);
}

#[test]
fn info_expoe_o_necessario_para_a_ui() {
    let info = profile("flac").unwrap().info();
    assert_eq!((info.id.as_str(), info.ext.as_str()), ("flac", "flac"));
    assert!(info.reencodes);
}

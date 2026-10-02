use super::*;

#[test]
fn previa_valida_modelo_e_reflete_organizacao_das_configs() {
    let dir = tempfile::tempdir().unwrap();
    let mut settings = crate::Settings {
        output_dir: dir.path().to_string_lossy().into_owned(),
        ..Default::default()
    };
    assert_eq!(
        PathBuf::from(preview("{artist}/{track:02} - {title}", &settings).unwrap()),
        PathBuf::from("Rick Astley/01 - Never Gonna Give You Up.opus")
    );
    assert!(preview("{desconhecido}", &settings).is_err());
    settings.auto_organize = false;
    assert_eq!(
        preview("{artist}/{title}", &settings).unwrap(),
        "Rick Astley - Never Gonna Give You Up.opus"
    );
    assert!(preview("{desconhecido}", &settings).is_err());
}

fn context(root: &Path) -> TemplateContext {
    TemplateContext {
        output_dir: root.to_owned(),
        extension: "opus".into(),
        language: Language::PtBr,
        content_type: ContentType::Music,
        auto_organize: true,
        channel: Some("canal".into()),
        source_id: Some("abc123".into()),
        playlist: Some("Favoritas".into()),
        playlist_index: Some(7),
    }
}

fn tags() -> TrackTags {
    TrackTags {
        title: "Título".into(),
        artist: Some("Artista".into()),
        album_artist: Some("Álbum artista".into()),
        album: Some("Álbum".into()),
        track_no: Some(3),
        disc_no: Some(2),
        year: Some(2026),
        genre: Some("Rock".into()),
        ..Default::default()
    }
}

#[test]
fn t5_tabela_de_todas_as_variaveis_e_regras_por_componentes() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path());
    let full = tags();
    let empty = TrackTags {
        title: "Título".into(),
        ..Default::default()
    };
    let en = TemplateContext {
        language: Language::En,
        ..ctx.clone()
    };
    let other = TemplateContext {
        content_type: ContentType::Other,
        ..ctx.clone()
    };
    let flat = TemplateContext {
        auto_organize: false,
        ..ctx.clone()
    };
    let artist_fallback = TrackTags {
        album_artist: None,
        ..full.clone()
    };
    let other_en = TemplateContext {
        language: Language::En,
        ..other.clone()
    };
    let other_flat = TemplateContext {
        auto_organize: false,
        ..other.clone()
    };
    let cases: Vec<(&str, &TrackTags, &TemplateContext, Vec<&str>)> = vec![
        (
            "{albumartist}/{album}/{track:02} - {title}",
            &full,
            &ctx,
            vec!["Álbum artista", "Álbum", "03 - Título.opus"],
        ),
        (
            "{artist}/{title}",
            &full,
            &ctx,
            vec!["Artista", "Título.opus"],
        ),
        (
            "{albumartist}/{title}",
            &artist_fallback,
            &ctx,
            vec!["Artista", "Título.opus"],
        ),
        (
            "{album}/{title}",
            &empty,
            &ctx,
            vec!["Singles", "Título.opus"],
        ),
        (
            "{artist}/{title}",
            &empty,
            &ctx,
            vec!["Artista desconhecido", "Título.opus"],
        ),
        (
            "{albumartist}/{title}",
            &empty,
            &en,
            vec!["Unknown artist", "Título.opus"],
        ),
        ("{track:02} - {title}", &empty, &ctx, vec!["Título.opus"]),
        ("{title} - {year}", &empty, &ctx, vec!["Título.opus"]),
        (
            "{title} - {year} - fim",
            &empty,
            &ctx,
            vec!["Título - fim.opus"],
        ),
        ("{genre}/{title}", &empty, &ctx, vec!["Título.opus"]),
        (
            "{track}/{disc}/{year}/{genre}/{title}",
            &full,
            &ctx,
            vec!["3", "2", "2026", "Rock", "Título.opus"],
        ),
        (
            "{channel}/{source_id}/{playlist}/{playlist_index:03} - {title}",
            &full,
            &ctx,
            vec!["canal", "abc123", "Favoritas", "007 - Título.opus"],
        ),
        (
            "{playlist_index} - {title}",
            &full,
            &ctx,
            vec!["7 - Título.opus"],
        ),
        (
            "ignorado",
            &full,
            &other,
            vec!["Outros", "canal", "Título.opus"],
        ),
        (
            "ignorado",
            &full,
            &other_en,
            vec!["Other", "canal", "Título.opus"],
        ),
        ("{inválido}", &full, &flat, vec!["Artista - Título.opus"]),
        (
            "ignorado",
            &full,
            &other_flat,
            vec!["Outros", "canal", "Título.opus"],
        ),
        (
            "CON/NUL/{title}",
            &full,
            &ctx,
            vec!["_CON", "_NUL", "Título.opus"],
        ),
        (
            "{artist}\\{title}",
            &full,
            &ctx,
            vec!["Artista", "Título.opus"],
        ),
        (
            "{track} - {disc} - {title}",
            &empty,
            &ctx,
            vec!["Título.opus"],
        ),
    ];
    for (model, tags, ctx, expected) in cases {
        let path = render(model, tags, ctx).unwrap();
        let actual: Vec<_> = path
            .strip_prefix(dir.path())
            .unwrap()
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect();
        assert_eq!(actual, expected, "{model}");
    }
}

#[test]
fn metadados_nao_criam_diretorios_e_sao_sanitizados() {
    let dir = tempfile::tempdir().unwrap();
    let mut tags = tags();
    tags.artist = Some("AC/DC\\live".into());
    tags.title = "Oi: <x>\"|?*\n... ".into();
    let path = render("{artist}/{title}", &tags, &context(dir.path())).unwrap();
    assert_eq!(
        path.strip_prefix(dir.path()).unwrap(),
        Path::new("AC_DC_live").join("Oi - _x______.opus")
    );
}

#[test]
fn unicode_300_caracteres_extensao_e_colisao_respeitam_limites() {
    let dir = tempfile::tempdir().unwrap();
    let tags = TrackTags {
        title: "界".repeat(300),
        ..Default::default()
    };
    let ctx = context(dir.path());
    let first = render("{title}", &tags, &ctx).unwrap();
    assert_eq!(
        first.file_name().unwrap().to_string_lossy().chars().count(),
        MAX_COMPONENT_CHARS
    );
    std::fs::write(&first, "ocupado").unwrap();
    let second = render("{title}", &tags, &ctx).unwrap();
    assert!(second
        .file_name()
        .unwrap()
        .to_string_lossy()
        .ends_with(" (2).opus"));
    assert_eq!(
        second
            .file_name()
            .unwrap()
            .to_string_lossy()
            .chars()
            .count(),
        MAX_COMPONENT_CHARS
    );
    std::fs::write(&second, "ocupado").unwrap();
    assert!(render("{title}", &tags, &ctx)
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .ends_with(" (3).opus"));
}

#[test]
fn caminho_total_encurta_titulo_e_colisao_e_rejeita_raiz_impossivel() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir
        .path()
        .join("d".repeat(145usize.saturating_sub(dir.path().to_string_lossy().chars().count())));
    std::fs::create_dir_all(&root).unwrap();
    let ctx = context(&root);
    let tags = TrackTags {
        title: "界".repeat(300),
        ..Default::default()
    };
    let path = render("{title}", &tags, &ctx).unwrap();
    assert_eq!(path.to_string_lossy().chars().count(), MAX_PATH_CHARS);
    std::fs::write(&path, "ocupado").unwrap();
    assert_eq!(
        render("{title}", &tags, &ctx)
            .unwrap()
            .to_string_lossy()
            .chars()
            .count(),
        MAX_PATH_CHARS
    );
    assert!(render("{title}", &tags, &context(&PathBuf::from("x".repeat(240)))).is_err());
}

#[test]
fn nomes_reservados_com_extensao_e_espacos_e_vazios() {
    let dir = tempfile::tempdir().unwrap();
    let tags = TrackTags {
        title: "CON.txt".into(),
        artist: Some("  ".into()),
        album: Some(" ".into()),
        album_artist: Some(" ".into()),
        ..Default::default()
    };
    assert_eq!(
        render("{albumartist}/{album}/{title}", &tags, &context(dir.path())).unwrap(),
        dir.path()
            .join("Artista desconhecido")
            .join("Singles")
            .join("_CON.txt.opus")
    );
    assert_eq!(
        super::super::sanitize_component(&format!("CON.{}", "x".repeat(300)))
            .chars()
            .count(),
        120
    );
}

#[test]
fn rejeita_modelo_invalido_sem_criar_arquivos() {
    let dir = tempfile::tempdir().unwrap();
    for model in [
        "",
        "{title",
        "{unknown}",
        "{title:02}",
        "{track:abc}",
        "{title}}",
    ] {
        assert!(
            render(model, &tags(), &context(dir.path())).is_err(),
            "{model}"
        );
    }
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn truncar_titulo_nao_produz_nome_reservado() {
    // Sobra espaço para três letras + extensão: CONcert não pode virar CON.opus.
    let root = PathBuf::from("r".repeat(MAX_PATH_CHARS - 1 - 3 - 5));
    let tags = TrackTags {
        title: "CONcert".into(),
        ..Default::default()
    };
    let path = render("{title}", &tags, &context(&root)).unwrap();
    assert_eq!(path.file_name().unwrap(), "_CO.opus");
    assert_eq!(path.to_string_lossy().chars().count(), MAX_PATH_CHARS);
}

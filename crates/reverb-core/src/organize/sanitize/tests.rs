use super::*;

#[test]
fn tabela_de_componentes() {
    let casos = [
        ("CON", "_CON"),
        ("con", "_con"),
        ("aux.txt", "_aux.txt"),
        ("COM1", "_COM1"),
        ("lpt9.mp3", "_lpt9.mp3"),
        ("CONSOLE", "CONSOLE"),
        ("a:b", "a -b"),
        ("Artista: Título", "Artista - Título"),
        ("a?b*", "a_b_"),
        ("a<b>c\"d|e", "a_b_c_d_e"),
        ("nome.", "nome"),
        ("nome ", "nome"),
        ("nome. . ", "nome"),
        ("  nome", "nome"),
        ("a\\b/c", "a_b_c"),
        ("tab\there\nnova", "tab_here_nova"),
        ("", "_"),
        ("...", "_"),
        ("   ", "_"),
        ("Música 🎵 ao vivo 😀", "Música 🎵 ao vivo 😀"),
        (".oculto", ".oculto"),
    ];
    for (input, expected) in casos {
        assert_eq!(sanitize_component(input), expected, "{input:?}");
    }
}

#[test]
fn normaliza_acentos_nfd_para_nfc() {
    let nfd = "Cafe\u{301} Tacvba a\u{303}";
    let out = sanitize_component(nfd);
    assert_eq!(out, "Café Tacvba ã");
    assert_eq!(out.chars().count(), 13);
}

#[test]
fn limita_a_120_caracteres_sem_quebrar_unicode() {
    let longo = "ã".repeat(300);
    let out = sanitize_component(&longo);
    assert_eq!(out.chars().count(), MAX_COMPONENT_CHARS);
    assert!(out.chars().all(|c| c == 'ã'));

    let emoji = "😀".repeat(200);
    assert_eq!(sanitize_component(&emoji).chars().count(), 63);
    assert_eq!(sanitize_component(&emoji).len(), 252);

    // O corte que termina em ponto/espaço é limpo de novo.
    let com_ponto = format!("{}.{}", "a".repeat(119), "b".repeat(50));
    assert_eq!(sanitize_component(&com_ponto), "a".repeat(119));
}

#[test]
fn sanitize_path_monta_o_caminho_e_a_extensao() {
    let base = Path::new("saida");
    let path = sanitize_path(base, &["Rick: Astley", "Álbum?", "01 - Faixa."], "opus");
    assert_eq!(
        path,
        Path::new("saida")
            .join("Rick - Astley")
            .join("Álbum_")
            .join("01 - Faixa.opus")
    );
}

#[test]
fn sanitize_path_trata_nome_reservado_no_arquivo() {
    let path = sanitize_path(Path::new("o"), &["CON"], "mp3");
    assert_eq!(path, Path::new("o").join("_CON.mp3"));
}

#[test]
fn sanitize_path_encurta_o_titulo_para_240_caracteres() {
    let base = PathBuf::from(format!(
        "C:\\Users\\Jean Carlos de Souza\\{}",
        "Pasta muito longa ".repeat(8)
    ));
    let titulo = "Título muito longo ".repeat(30);
    let path = sanitize_path(&base, &["Artista", "Álbum", &titulo], "opus");
    let len = path.to_string_lossy().chars().count();
    assert!(len <= MAX_PATH_CHARS, "{len}");
    assert!(len > MAX_PATH_CHARS - 5, "deve aproveitar o limite: {len}");
    assert!(path.to_string_lossy().ends_with(".opus"));
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    assert!(!name.trim_end_matches(".opus").ends_with(' '));
}

#[test]
fn sanitize_path_com_base_enorme_ainda_devolve_um_nome() {
    let base = PathBuf::from("x".repeat(300));
    let path = sanitize_path(&base, &["Título"], "mp3");
    assert!(path.to_string_lossy().ends_with(".mp3"));
    assert!(path.file_name().unwrap().to_string_lossy().chars().count() >= 5);
}

#[test]
fn unique_path_acrescenta_sufixo() {
    let dir = tempfile::tempdir().unwrap();
    let alvo = dir.path().join("Faixa.opus");
    assert_eq!(unique_path(&alvo), alvo);

    std::fs::write(&alvo, b"1").unwrap();
    assert_eq!(unique_path(&alvo), dir.path().join("Faixa (2).opus"));

    std::fs::write(dir.path().join("Faixa (2).opus"), b"2").unwrap();
    assert_eq!(unique_path(&alvo), dir.path().join("Faixa (3).opus"));
}

#[test]
fn unique_path_sem_extensao() {
    let dir = tempfile::tempdir().unwrap();
    let alvo = dir.path().join("Faixa");
    std::fs::write(&alvo, b"1").unwrap();
    assert_eq!(unique_path(&alvo), dir.path().join("Faixa (2)"));
}

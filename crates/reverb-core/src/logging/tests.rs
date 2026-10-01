use super::*;
use tempfile::tempdir;

#[test]
fn t11_mascara_query_strings_sensiveis() {
    assert_eq!(redact("GET /x?key=ABC123&x=1"), "GET /x?key=***&x=1");
    assert_eq!(redact("client_secret=xyz"), "client_secret=***");
    assert_eq!(
        redact("url?token=abc&sig=def&signature=ghi"),
        "url?token=***&sig=***&signature=***"
    );
    assert_eq!(redact("?api_key=SEGREDO1"), "?api_key=***");
    assert_eq!(redact("access_token=abcdef"), "access_token=***");
}

#[test]
fn t11_mascara_cabecalhos_e_segredos_conhecidos() {
    assert_eq!(
        redact("Authorization: Bearer abc.def"),
        "Authorization: ***"
    );
    assert_eq!(redact("Cookie: SID=1; HSID=2"), "Cookie: ***");
    register_secret("chave-secreta-unica-t11");
    assert_eq!(
        redact("usando chave-secreta-unica-t11 agora"),
        "usando *** agora"
    );
}

#[test]
fn t11_texto_comum_fica_intacto() {
    let text = "Baixando \"Never Gonna Give You Up\" (214 s) para D:\\Músicas — ok";
    assert_eq!(redact(text), text);
    assert_eq!(redact("monkey business"), "monkey business");
}

#[test]
fn segredos_curtos_nao_sao_registrados() {
    register_secret("ab");
    assert_eq!(redact("abacaxi ab"), "abacaxi ab");
}

#[test]
fn writer_redige_o_que_escreve() {
    let mut out = Vec::new();
    {
        let mut writer = RedactingWriter(&mut out);
        writer.write_all(b"GET /x?key=ABC123\n").unwrap();
    }
    assert_eq!(String::from_utf8(out).unwrap(), "GET /x?key=***\n");
}

#[test]
fn prune_mantem_os_mais_recentes() {
    let dir = tempdir().unwrap();
    for day in 1..=20 {
        std::fs::write(
            dir.path()
                .join(format!("{LOG_FILE_PREFIX}.2026-09-{day:02}")),
            "x",
        )
        .unwrap();
    }
    std::fs::write(dir.path().join("outro.txt"), "x").unwrap();

    assert_eq!(prune_old_logs(dir.path(), 14).unwrap(), 6);

    let mut names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names.len(), 15); // 14 logs + outro.txt (não é log)
    assert!(names.contains(&format!("{LOG_FILE_PREFIX}.2026-09-20")));
    assert!(!names.contains(&format!("{LOG_FILE_PREFIX}.2026-09-06")));
    assert!(names.contains(&"outro.txt".to_string()));
}

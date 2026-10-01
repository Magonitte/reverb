//! T12 — `init` é global por processo, por isso vive em um teste de integração próprio.

use reverb_core::logging::{self, LOG_FILE_PREFIX, LOG_RETENTION};

#[test]
fn t12_init_aplica_retencao_de_14_arquivos_e_redige_o_log() {
    let dir = tempfile::tempdir().unwrap();
    for day in 1..=20 {
        std::fs::write(
            dir.path()
                .join(format!("{LOG_FILE_PREFIX}.2020-01-{day:02}")),
            "antigo",
        )
        .unwrap();
    }

    let guard = logging::init(dir.path()).expect("init");

    let count = |dir: &std::path::Path| {
        std::fs::read_dir(dir)
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(LOG_FILE_PREFIX)
            })
            .count()
    };
    assert_eq!(count(dir.path()), LOG_RETENTION);

    tracing::info!("requisição https://exemplo.test/x?key=ABC123&x=1");
    drop(guard); // descarrega o worker não bloqueante

    let mut content = String::new();
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let path = entry.unwrap().path();
        if !path.to_string_lossy().contains("2020-01") {
            content.push_str(&std::fs::read_to_string(path).unwrap());
        }
    }
    assert!(content.contains("key=***&x=1"), "log: {content}");
    assert!(
        !content.contains("ABC123"),
        "log vazou o segredo: {content}"
    );
}

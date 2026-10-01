use super::*;

fn installed(version: &str) -> Installed {
    Installed {
        version: version.into(),
        reported_version: Some(version.into()),
        channel: Some("stable".into()),
        installed_at: "2026-10-01T00:00:00Z".into(),
        asset_updated_at: None,
    }
}

#[test]
fn salva_e_recarrega() {
    let dir = tempfile::tempdir().unwrap();
    let mut manifest = Manifest::default();
    manifest.tools.insert(
        "ytdlp".into(),
        ToolEntry {
            current: Some(installed("2026.10.02")),
            previous: Some(installed("2026.10.01")),
        },
    );
    manifest.save(dir.path()).unwrap();

    let loaded = Manifest::load(dir.path());
    assert_eq!(loaded, manifest);
    assert_eq!(loaded.current("ytdlp").unwrap().version, "2026.10.02");
    assert!(loaded.current("deno").is_none());
}

#[test]
fn gravacao_atomica_nao_deixa_temporario() {
    let dir = tempfile::tempdir().unwrap();
    Manifest::default().save(dir.path()).unwrap();
    Manifest::default().save(dir.path()).unwrap();
    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, [MANIFEST_FILE]);
}

#[test]
fn ausente_ou_corrompido_vira_vazio() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(Manifest::load(dir.path()), Manifest::default());
    std::fs::write(Manifest::path(dir.path()), "{ isto não é json").unwrap();
    assert_eq!(Manifest::load(dir.path()), Manifest::default());
}

#[test]
fn cria_a_pasta_se_preciso() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("a").join("b");
    Manifest::default().save(&nested).unwrap();
    assert!(Manifest::path(&nested).is_file());
}

#[test]
fn data_rfc3339() {
    assert_eq!(rfc3339_from_secs(0), "1970-01-01T00:00:00Z");
    assert_eq!(rfc3339_from_secs(951_782_400), "2000-02-29T00:00:00Z");
    assert_eq!(rfc3339_from_secs(1_000_000_000), "2001-09-09T01:46:40Z");
    assert_eq!(
        rfc3339_from_secs(1_782_864_000 + 3661),
        "2026-07-01T01:01:01Z"
    );
    let now = now_rfc3339();
    assert_eq!(now.len(), 20);
    assert!(now.ends_with('Z'));
}

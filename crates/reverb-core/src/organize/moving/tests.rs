use super::*;

#[test]
fn t6_copia_entre_volumes_preserva_bytes_remove_origem_e_limpa_staging() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.opus");
    let target = dir.path().join("library").join("final.opus");
    let bytes: Vec<u8> = (0..200_000).map(|n| (n % 251) as u8).collect();
    std::fs::write(&source, &bytes).unwrap();
    assert_eq!(
        move_into_library_with_mode(&source, &target, MoveMode::Copy).unwrap(),
        target
    );
    assert_eq!(std::fs::read(&target).unwrap(), bytes);
    assert!(!source.exists());
    assert_eq!(
        std::fs::read_dir(target.parent().unwrap()).unwrap().count(),
        1
    );
}

#[test]
fn rename_e_copia_nunca_sobrescrevem_destino() {
    for mode in [MoveMode::Auto, MoveMode::Copy] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.opus");
        let target = dir.path().join("target.opus");
        std::fs::write(&source, "novo").unwrap();
        std::fs::write(&target, "antigo").unwrap();
        let actual = move_into_library_with_mode(&source, &target, mode).unwrap();
        assert_eq!(actual, dir.path().join("target (2).opus"));
        assert_eq!(std::fs::read_to_string(target).unwrap(), "antigo");
        assert_eq!(std::fs::read_to_string(actual).unwrap(), "novo");
        assert!(!source.exists());
    }
}

#[test]
fn falha_de_destino_preserva_origem_sem_staging() {
    for mode in [MoveMode::Auto, MoveMode::Copy] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.opus");
        let blocker = dir.path().join("arquivo");
        std::fs::write(&source, "original").unwrap();
        std::fs::write(&blocker, "bloqueio").unwrap();
        assert!(move_into_library_with_mode(&source, &blocker.join("target.opus"), mode).is_err());
        assert_eq!(std::fs::read_to_string(source).unwrap(), "original");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }
}

#[test]
fn origem_ausente_diretorio_ou_mesmo_arquivo_retorna_erro() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.opus");
    std::fs::write(&source, "original").unwrap();
    assert!(move_into_library(&source, &source).is_err());
    assert!(move_into_library(dir.path(), &dir.path().join("new.opus")).is_err());
    assert!(move_into_library(&dir.path().join("missing"), &dir.path().join("new.opus")).is_err());
    assert_eq!(std::fs::read_to_string(source).unwrap(), "original");
}

#[test]
fn publicacoes_concorrentes_escolhem_nomes_distintos_sem_perder_conteudo() {
    for mode in [MoveMode::Auto, MoveMode::Copy] {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("final.opus");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let threads: Vec<_> = (0..8)
            .map(|n| {
                let source = dir.path().join(format!("source-{n}"));
                std::fs::write(&source, n.to_string()).unwrap();
                let target = target.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    move_into_library_with_mode(&source, &target, mode).unwrap()
                })
            })
            .collect();
        let mut contents: Vec<_> = threads
            .into_iter()
            .map(|thread| std::fs::read_to_string(thread.join().unwrap()).unwrap())
            .collect();
        contents.sort();
        assert_eq!(contents, (0..8).map(|n| n.to_string()).collect::<Vec<_>>());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 8);
    }
}

#[cfg(windows)]
#[test]
fn falha_ao_remover_origem_desfaz_publicacao_da_copia() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("locked.opus");
    let target = dir.path().join("library").join("final.opus");
    std::fs::write(&source, "original").unwrap();
    // Compartilha leitura/escrita, mas não exclusão: força falha real no remove_file.
    let guard = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&source)
        .unwrap();
    let result = move_into_library_with_mode(&source, &target, MoveMode::Copy);
    drop(guard);
    assert!(result.is_err());
    assert_eq!(std::fs::read_to_string(source).unwrap(), "original");
    assert!(!target.exists());
    assert_eq!(
        std::fs::read_dir(target.parent().unwrap()).unwrap().count(),
        0
    );
}

#[cfg(windows)]
#[test]
fn rename_bloqueado_nao_faz_fallback_de_volume() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("locked.opus");
    let target = dir.path().join("final.opus");
    std::fs::write(&source, "original").unwrap();
    let guard = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(3)
        .open(&source)
        .unwrap();
    let result = move_into_library(&source, &target);
    drop(guard);
    assert!(result.is_err());
    assert_eq!(std::fs::read_to_string(source).unwrap(), "original");
    assert!(!target.exists());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

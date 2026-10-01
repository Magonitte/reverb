use std::time::Duration;

use super::*;

#[test]
fn cria_e_apaga_no_drop() {
    let data = tempfile::tempdir().unwrap();
    let ws = JobWorkspace::new(data.path(), "job-1").unwrap();
    let dir = ws.path().to_path_buf();
    assert_eq!(dir, data.path().join("tmp").join("job-1"));
    std::fs::write(dir.join("a.part"), b"x").unwrap();
    drop(ws);
    assert!(!dir.exists());
    assert!(data.path().join("tmp").is_dir(), "só a pasta do job some");
}

#[test]
fn rejeita_job_id_que_escapa_da_pasta() {
    let data = tempfile::tempdir().unwrap();
    for id in ["", ".", "..", "a/b", "a\\b", "c:x"] {
        assert!(JobWorkspace::new(data.path(), id).is_err(), "{id:?}");
    }
}

#[test]
fn sweep_remove_pastas_e_arquivos_antigos() {
    let data = tempfile::tempdir().unwrap();
    let tmp = data.path().join("tmp");
    std::fs::create_dir_all(tmp.join("velho-1/sub")).unwrap();
    std::fs::write(tmp.join("velho-1/sub/x.part"), b"x").unwrap();
    std::fs::create_dir_all(tmp.join("velho-2")).unwrap();
    std::fs::write(tmp.join("solto.txt"), b"x").unwrap();

    assert_eq!(sweep_orphans(data.path(), Duration::ZERO), 3);
    assert_eq!(std::fs::read_dir(&tmp).unwrap().count(), 0);
}

#[test]
fn sweep_preserva_o_que_e_recente() {
    let data = tempfile::tempdir().unwrap();
    let ws = JobWorkspace::new(data.path(), "em-uso").unwrap();
    assert_eq!(sweep_orphans(data.path(), Duration::from_secs(3600)), 0);
    assert!(ws.path().is_dir());
}

#[test]
fn sweep_sem_pasta_tmp_nao_falha() {
    let data = tempfile::tempdir().unwrap();
    assert_eq!(sweep_orphans(data.path(), Duration::ZERO), 0);
}

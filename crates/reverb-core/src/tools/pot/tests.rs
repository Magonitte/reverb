//! T12b: política do provedor de PO token e ciclo de vida do servidor (com o `fake-tool`).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use sysinfo::{Pid, ProcessStatus, ProcessesToUpdate, System};

use super::*;
use crate::tools::testutil::fake_tool_path;

// ------------------------------------------------------------------ montagem de argumentos

#[test]
fn argumentos_do_yt_dlp_com_o_provedor_ativo() {
    let args = pot_args(Path::new("/dados/tools/bgutil/2.0.0/plugins"), 4416);
    assert_eq!(
        args,
        [
            "--plugin-dirs",
            "/dados/tools/bgutil/2.0.0/plugins",
            "--extractor-args",
            "youtubepot-bgutilhttp:base_url=http://127.0.0.1:4416",
        ]
    );
    // Não força o player_client: o yt-dlp escolhe.
    assert!(!args.iter().any(|a| a.contains("player_client")));
}

// ------------------------------------------------------------------ política

fn clock(start: u64) -> (Arc<AtomicU64>, Clock) {
    let now = Arc::new(AtomicU64::new(start));
    let shared = Arc::clone(&now);
    (now, Arc::new(move || shared.load(Ordering::SeqCst)))
}

#[tokio::test]
async fn auto_comeca_desligado_liga_e_expira_em_24h() {
    let (now, clock) = clock(1_000_000);
    let policy = PotPolicy::with_clock(Db::open_in_memory().unwrap(), clock);

    assert!(!policy.is_active(PotProvider::Auto).await.unwrap());

    policy.enable_auto().await.unwrap(); // a autocura viu bot_check
    assert!(policy.is_active(PotProvider::Auto).await.unwrap());
    assert_eq!(
        policy.auto_until().await.unwrap(),
        Some(1_000_000 + AUTO_TTL_SECS)
    );

    now.store(1_000_000 + AUTO_TTL_SECS - 1, Ordering::SeqCst);
    assert!(policy.is_active(PotProvider::Auto).await.unwrap());
    now.store(1_000_000 + AUTO_TTL_SECS, Ordering::SeqCst);
    assert!(!policy.is_active(PotProvider::Auto).await.unwrap());
}

#[tokio::test]
async fn ligar_de_novo_renova_as_24h() {
    let (now, clock) = clock(0);
    let policy = PotPolicy::with_clock(Db::open_in_memory().unwrap(), clock);
    policy.enable_auto().await.unwrap();
    now.store(AUTO_TTL_SECS - 10, Ordering::SeqCst);
    policy.enable_auto().await.unwrap();
    now.store(AUTO_TTL_SECS + 100, Ordering::SeqCst);
    assert!(policy.is_active(PotProvider::Auto).await.unwrap());
}

#[tokio::test]
async fn always_e_off_ignoram_o_estado() {
    let (_now, clock) = clock(5);
    let policy = PotPolicy::with_clock(Db::open_in_memory().unwrap(), clock);
    assert!(policy.is_active(PotProvider::Always).await.unwrap());
    assert!(!policy.is_active(PotProvider::Off).await.unwrap());
    policy.enable_auto().await.unwrap();
    assert!(!policy.is_active(PotProvider::Off).await.unwrap());
}

#[tokio::test]
async fn estado_da_politica_sobrevive_ao_reinicio() {
    let (_now, clock) = clock(100);
    let db = Db::open_in_memory().unwrap();
    PotPolicy::with_clock(db.clone(), Arc::clone(&clock))
        .enable_auto()
        .await
        .unwrap();
    // Outra instância sobre o mesmo banco enxerga o estado.
    assert!(PotPolicy::with_clock(db, clock)
        .is_active(PotProvider::Auto)
        .await
        .unwrap());
}

// ------------------------------------------------------------------ servidor

fn config(idle: Duration) -> PotServerConfig {
    let args: ArgsFn = Arc::new(|port: u16| {
        vec![
            OsString::from("--http-ping"),
            OsString::from(port.to_string()),
        ]
    });
    let mut cfg = PotServerConfig::new(fake_tool_path(), args, None);
    cfg.idle_timeout = idle;
    cfg.idle_check = Duration::from_millis(50);
    cfg.ping_timeout = Duration::from_secs(15);
    cfg
}

fn pid_alive(pid: u32) -> bool {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    // Zumbi (morto, ainda não colhido pelo init do contêiner) conta como morto.
    system
        .process(Pid::from_u32(pid))
        .is_some_and(|p| p.status() != ProcessStatus::Zombie)
}

fn kill_pid(pid: u32) {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    system
        .process(Pid::from_u32(pid))
        .expect("processo do servidor")
        .kill();
}

async fn wait_until(limit: Duration, mut condition: impl FnMut() -> bool) -> bool {
    let started = Instant::now();
    while started.elapsed() < limit {
        if condition() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

async fn ping(port: u16) -> bool {
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    matches!(
        client.get(format!("http://127.0.0.1:{port}/ping")).send().await,
        Ok(r) if r.status().is_success()
    )
}

#[tokio::test]
async fn sobe_sob_demanda_e_reutiliza() {
    let server = PotServer::new(config(Duration::from_secs(60)));
    assert!(!server.is_running().await);

    let port = server.ensure_running().await.unwrap();
    assert!(ping(port).await);
    let pid = server.pid().await.unwrap();

    // Segunda chamada (outro job): mesma porta, mesmo processo.
    assert_eq!(server.ensure_running().await.unwrap(), port);
    assert_eq!(server.pid().await, Some(pid));

    server.stop().await;
    assert!(!server.is_running().await);
    assert!(wait_until(Duration::from_secs(3), || !pid_alive(pid)).await);
}

#[tokio::test]
async fn encerra_por_ociosidade_e_sobe_de_novo_quando_precisa() {
    let server = PotServer::new(config(Duration::from_millis(400)));
    let port = server.ensure_running().await.unwrap();
    let pid = server.pid().await.unwrap();

    // Usar o servidor adia o encerramento.
    for _ in 0..4 {
        tokio::time::sleep(Duration::from_millis(150)).await;
        server.ensure_running().await.unwrap();
    }
    assert!(server.is_running().await);

    // Sem uso: encerra a árvore inteira.
    assert!(
        wait_until(Duration::from_secs(5), || !pid_alive(pid)).await,
        "o servidor ocioso deveria ter sido encerrado"
    );
    assert!(!server.is_running().await);
    assert!(!ping(port).await);

    // Novo uso ⇒ novo processo (ociosidade não conta como morte).
    let new_port = server.ensure_running().await.unwrap();
    assert!(ping(new_port).await);
    assert!(!server.gave_up().await);
    server.stop().await;
}

#[tokio::test]
async fn reinicia_uma_vez_se_morrer_e_desiste_na_segunda_morte() {
    let server = PotServer::new(config(Duration::from_secs(60)));
    server.ensure_running().await.unwrap();
    let first = server.pid().await.unwrap();

    // 1ª morte ⇒ reinicia.
    kill_pid(first);
    assert!(wait_until(Duration::from_secs(5), || !pid_alive(first)).await);
    let port = server.ensure_running().await.unwrap();
    assert!(ping(port).await);
    let second = server.pid().await.unwrap();
    assert_ne!(first, second);

    // 2ª morte ⇒ desiste (segue sem token).
    kill_pid(second);
    assert!(wait_until(Duration::from_secs(5), || !pid_alive(second)).await);
    let err = server.ensure_running().await.unwrap_err();
    assert_eq!(err.kind(), "pot_unavailable");
    assert!(server.gave_up().await);

    // Continua desistindo, sem tentar subir de novo.
    assert!(server.ensure_running().await.is_err());
    assert!(!server.is_running().await);

    // `stop` (troca de versão do bgutil / fim do app) zera o estado.
    server.stop().await;
    assert!(!server.gave_up().await);
    assert!(server.ensure_running().await.is_ok());
    server.stop().await;
}

#[tokio::test]
async fn processo_que_nao_sobe_desiste_apos_duas_tentativas() {
    let args: ArgsFn = Arc::new(|_| vec![OsString::from("--exit"), OsString::from("1")]);
    let mut cfg = PotServerConfig::new(fake_tool_path(), args, None);
    cfg.ping_timeout = Duration::from_secs(5);
    let server = PotServer::new(cfg);

    let started = Instant::now();
    let err = server.ensure_running().await.unwrap_err();
    assert_eq!(err.kind(), "pot_unavailable");
    assert!(server.gave_up().await);
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[tokio::test]
async fn servidor_que_nao_responde_ao_ping_e_morto_pelo_timeout() {
    // Um processo vivo que não abre porta alguma.
    let args: ArgsFn = Arc::new(|_| vec![OsString::from("--sleep"), OsString::from("30")]);
    let mut cfg = PotServerConfig::new(fake_tool_path(), args, None);
    cfg.ping_timeout = Duration::from_millis(600);
    let server = PotServer::new(cfg);

    let err = server.ensure_running().await.unwrap_err();
    assert_eq!(err.kind(), "pot_unavailable");
    assert!(!server.is_running().await);
}

//! Atualização do app (arquitetura §17, fase F06).
//!
//! A decisão (intervalo, interruptor, semver) é pura e testada aqui. Baixar e
//! instalar usa o `tauri-plugin-updater`, configurado no `tauri.conf.json`.

use std::path::Path;
use std::time::{Duration, SystemTime};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

use crate::headless::UpdateMode;
use crate::state::AppState;

/// Espera depois da abertura antes da primeira verificação automática.
pub const STARTUP_DELAY: Duration = Duration::from_secs(10);
/// Intervalo entre verificações automáticas, se `autoCheckAppUpdates`.
pub const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateInfo {
    pub version: String,
    pub current_version: String,
    pub notes: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

/// `true` quando a verificação automática deve rodar agora.
///
/// Desligada nunca verifica. Sem verificação anterior, verifica. Com uma
/// anterior, só depois de [`CHECK_INTERVAL`]. Relógio andando para trás espera.
pub fn should_check(enabled: bool, last_check: Option<SystemTime>, now: SystemTime) -> bool {
    if !enabled {
        return false;
    }
    match last_check {
        None => true,
        Some(last) => match now.duration_since(last) {
            Ok(elapsed) => elapsed >= CHECK_INTERVAL,
            Err(_) => false,
        },
    }
}

/// A versão anunciada é atualização só se for semver estritamente maior.
/// Aceita o prefixo `v`. Versão inválida não é atualização.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    let parse = |raw: &str| semver::Version::parse(raw.trim().trim_start_matches('v')).ok();
    match (parse(candidate), parse(current)) {
        (Some(next), Some(now)) => next > now,
        _ => false,
    }
}

pub async fn check(app: &AppHandle) -> Result<Option<AppUpdateInfo>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    match updater.check().await.map_err(|e| e.to_string())? {
        None => Ok(None),
        Some(update) if !is_newer(&update.version, &update.current_version) => Ok(None),
        Some(update) => Ok(Some(AppUpdateInfo {
            version: update.version.clone(),
            current_version: update.current_version.clone(),
            notes: update.body.clone(),
            date: update.date.map(|d| d.to_string()),
        })),
    }
}

/// Baixa, verifica a assinatura e instala. No Windows o instalador encerra o
/// processo. No Linux, `restart_app` chama `app.restart()` depois de instalar.
pub async fn install(app: &AppHandle, restart_app: bool) -> Result<(), String> {
    if let Some(state) = app.try_state::<AppState>() {
        state.queue.shutdown().await;
        state.tools.shutdown().await;
    }
    let updater = app.updater().map_err(|e| e.to_string())?;
    let Some(update) = updater.check().await.map_err(|e| e.to_string())? else {
        return Err("não há atualização para instalar".into());
    };
    let update = update.restart_after_install(restart_app);
    let handle = app.clone();
    let mut downloaded: u64 = 0;
    update
        .download_and_install(
            move |chunk, total| {
                downloaded = downloaded.saturating_add(chunk as u64);
                let _ = handle.emit("updater://progress", UpdateProgress { downloaded, total });
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;

    #[cfg(windows)]
    {
        let _ = restart_app;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        if restart_app {
            app.restart();
        } else {
            Ok(())
        }
    }
}

/// Verificação automática: 10 s após abrir e depois a cada 6 h, se ligado.
pub fn spawn_auto_check(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(STARTUP_DELAY).await;
        let mut last: Option<SystemTime> = None;
        loop {
            let enabled = app
                .try_state::<AppState>()
                .is_some_and(|state| state.settings.get().auto_check_app_updates);
            let now = SystemTime::now();
            if should_check(enabled, last, now) {
                last = Some(now);
                if app.updater().is_err() {
                    tracing::debug!("verificação automática adiada: atualizador sem endpoint");
                } else {
                    match check(&app).await {
                        Ok(Some(info)) => {
                            let _ = app.emit("updater://available", &info);
                        }
                        Ok(None) => {}
                        Err(e) => {
                            tracing::warn!(error = %e, "verificação automática de atualização falhou");
                        }
                    }
                }
            }
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}

pub async fn run_headless(app: &AppHandle, mode: &UpdateMode) -> i32 {
    let result = match mode {
        UpdateMode::Check(path) => write_check(app, path).await,
        UpdateMode::Install => install(app, false).await,
    };
    match result {
        Ok(()) => 0,
        Err(message) => {
            eprintln!("atualização headless: {message}");
            1
        }
    }
}

async fn write_check(app: &AppHandle, path: &Path) -> Result<(), String> {
    let current = app.package_info().version.to_string();
    let update = check(app).await?;
    let mut body = serde_json::json!({
        "currentVersion": current,
        "available": update.is_some(),
    });
    if let Some(info) = update {
        body["version"] = serde_json::Value::String(info.version);
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    std::fs::write(path, body.to_string()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn intervalo_e_atraso_combinam_com_o_plano() {
        assert_eq!(STARTUP_DELAY, Duration::from_secs(10));
        assert_eq!(CHECK_INTERVAL, Duration::from_secs(6 * 60 * 60));
    }

    #[test]
    fn desligado_nao_verifica() {
        let now = at(1_000_000);
        assert!(!should_check(false, None, now));
        let last = now.checked_sub(CHECK_INTERVAL).unwrap();
        assert!(!should_check(false, Some(last), now));
    }

    #[test]
    fn ligado_verifica_na_primeira_vez_e_so_depois_do_intervalo() {
        let now = at(1_000_000);
        assert!(should_check(true, None, now));
        let recente = now
            .checked_sub(CHECK_INTERVAL - Duration::from_secs(1))
            .unwrap();
        assert!(!should_check(true, Some(recente), now));
        let vencido = now.checked_sub(CHECK_INTERVAL).unwrap();
        assert!(should_check(true, Some(vencido), now));
        assert!(!should_check(true, Some(now + Duration::from_secs(5)), now));
    }

    #[test]
    fn semver_so_aceita_versao_estritamente_maior() {
        assert!(is_newer("0.1.1", "0.1.0"));
        assert!(is_newer("v1.2.0", "1.1.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(!is_newer("nao-e-versao", "0.1.0"));
        assert!(!is_newer("0.2.0", ""));
    }
}

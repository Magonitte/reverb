//! Comandos da F07: classificar/analisar/buscar e as ações de sistema (pasta, área de
//! transferência, revelar arquivo). Toda interação com o SO fica aqui, em Rust (protocolo §7.3).

use std::path::{Path, PathBuf};

use reverb_core::paths::resolve_output_dir;
use reverb_core::urlkind::{classify, UrlKind};
use reverb_core::ytdlp::errors::DownloadError;
use reverb_core::ytdlp::{Analysis, SearchResult, SearchSource};
use reverb_core::CoreError;
use tauri::{AppHandle, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use tokio_util::sync::CancellationToken;

use crate::state::AppState;

/// Quantos resultados cada busca pede ao yt-dlp.
const SEARCH_LIMIT: u32 = 15;

/// Erro do motor ⇒ erro de comando com o `kind` do `ErrorKind` (a UI traduz `errors.<kind>`).
pub(crate) fn engine_error(error: DownloadError) -> CoreError {
    if error.message == "errors.bandcampRestricted" {
        return CoreError::coded("bandcamp_restricted", error.message);
    }
    if error.message == "Jamendo client ID required" {
        return CoreError::coded("jamendo_key", error.message);
    }
    let kind = error.kind.as_str();
    CoreError::coded(kind, error.message)
}

#[tauri::command]
pub fn url_classify(input: String) -> UrlKind {
    classify(&input)
}

#[tauri::command]
pub async fn analyze(state: State<'_, AppState>, url: String) -> Result<Analysis, CoreError> {
    state
        .backend
        .analyze(&url, &CancellationToken::new())
        .await
        .map_err(engine_error)
}

#[tauri::command]
pub async fn search(
    state: State<'_, AppState>,
    source: String,
    query: String,
) -> Result<Vec<SearchResult>, CoreError> {
    let source = SearchSource::from_id(&source)
        .ok_or_else(|| CoreError::invalid(format!("fonte de busca desconhecida: {source}")))?;
    state
        .backend
        .search(source, &query, SEARCH_LIMIT, &CancellationToken::new())
        .await
        .map_err(engine_error)
}

/// Seletor de pasta nativo; `None` se o usuário cancelou.
#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Result<Option<String>, CoreError> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |path| {
        let _ = tx.send(path);
    });
    let picked = rx
        .await
        .map_err(|_| CoreError::Internal("o seletor de pasta foi fechado".into()))?;
    match picked {
        None => Ok(None),
        Some(path) => {
            let path = path
                .into_path()
                .map_err(|e| CoreError::Internal(e.to_string()))?;
            Ok(Some(path.to_string_lossy().into_owned()))
        }
    }
}

/// Abre a pasta de destino (a configurada ou a padrão), criando-a se faltar.
#[tauri::command]
pub fn open_output_dir(app: AppHandle, state: State<'_, AppState>) -> Result<(), CoreError> {
    let dir = resolve_output_dir(&state.settings.get());
    std::fs::create_dir_all(&dir)?;
    open_path(&app, &dir)
}

#[tauri::command]
pub fn clipboard_read_text(app: AppHandle) -> Result<String, CoreError> {
    // Área de transferência vazia ou sem texto não é erro para a UI.
    Ok(app.clipboard().read_text().unwrap_or_default())
}

/// Mostra o arquivo no gerenciador de arquivos (F10 passa a aceitar também o id da biblioteca).
#[tauri::command]
pub fn library_reveal(app: AppHandle, path: String) -> Result<(), CoreError> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(CoreError::coded(
            "file_missing",
            format!("arquivo não encontrado: {}", path.display()),
        ));
    }
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| CoreError::Internal(e.to_string()))
}

fn open_path(app: &AppHandle, path: &Path) -> Result<(), CoreError> {
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| CoreError::Internal(e.to_string()))
}

#[tauri::command]
pub fn library_open_file(app: AppHandle, path: String) -> Result<(), CoreError> {
    let path = PathBuf::from(path);
    if !path.is_file() {
        return Err(CoreError::coded(
            "file_missing",
            "Arquivo de áudio não encontrado",
        ));
    }
    open_path(&app, &path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reverb_core::ytdlp::errors::ErrorKind;

    #[test]
    fn erro_do_motor_vira_o_kind_que_a_ui_traduz() {
        for kind in [
            ErrorKind::Cancelled,
            ErrorKind::Unavailable,
            ErrorKind::AgeRestricted,
            ErrorKind::BotCheck,
            ErrorKind::GeoBlocked,
            ErrorKind::Disk,
            ErrorKind::Ffmpeg,
            ErrorKind::Extractor,
            ErrorKind::Network,
            ErrorKind::Unknown,
        ] {
            let error = engine_error(DownloadError::new(kind, "mensagem crua"));
            assert_eq!(error.kind(), kind.as_str());
            assert_eq!(error.to_string(), "mensagem crua");
        }
    }

    #[test]
    fn url_classify_delega_ao_core() {
        assert!(matches!(
            url_classify("https://youtu.be/dQw4w9WgXcQ".into()),
            UrlKind::Video { .. }
        ));
        assert!(matches!(
            url_classify("rick astley".into()),
            UrlKind::Search { .. }
        ));
        assert!(matches!(
            url_classify("https://vimeo.com/1".into()),
            UrlKind::Unsupported
        ));
    }
}

#[tauri::command]
pub async fn cookies_test(
    state: State<'_, AppState>,
) -> Result<reverb_core::quality::CookiesTestResult, CoreError> {
    Ok(reverb_core::quality::test_cookies(state.backend.as_ref(), &CancellationToken::new()).await)
}

//! Comandos da F08: versão oficial, pré-visualização de metadados (simulação sem baixar) e busca
//! manual nos provedores.

use reverb_core::metadata::{Candidate, MetadataResult, OfficialMatch, PreviewRequest};
use reverb_core::ytdlp::VideoInfo;
use reverb_core::CoreError;
use tauri::State;
use tokio_util::sync::CancellationToken;

use super::media::engine_error;
use crate::state::AppState;

/// A faixa oficial de um vídeo, se existir (E1: ISRC, depois texto com filtros eliminatórios).
#[tauri::command]
pub async fn find_official_version(
    state: State<'_, AppState>,
    video: VideoInfo,
    isrc: Option<String>,
) -> Result<Option<OfficialMatch>, CoreError> {
    Ok(state
        .metadata
        .find_official_version(&video, isrc.as_deref(), &CancellationToken::new())
        .await)
}

/// O que o pipeline faria com a URL, sem baixar nada (usado no Preview).
#[tauri::command]
pub async fn metadata_preview(
    state: State<'_, AppState>,
    request: PreviewRequest,
) -> Result<MetadataResult, CoreError> {
    state
        .metadata
        .preview(
            &request.url,
            request.video,
            request.use_official,
            &CancellationToken::new(),
        )
        .await
        .map_err(engine_error)
}

/// Busca manual nos provedores (Deezer, iTunes, MusicBrainz).
#[tauri::command]
pub async fn metadata_search(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<Candidate>, CoreError> {
    Ok(state.metadata.search(&query).await)
}

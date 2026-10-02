use crate::state::AppState;
use reverb_core::error::ErrorPayload;
use reverb_core::sync::{Sync, SyncCreate, SyncItem, SyncResult, SyncUpdate};
use tauri::State;

#[tauri::command]
pub async fn syncs_list(state: State<'_, AppState>) -> Result<Vec<Sync>, ErrorPayload> {
    state
        .syncs
        .list()
        .await
        .map_err(|error| ErrorPayload::from(&error))
}
#[tauri::command]
pub async fn sync_create(
    state: State<'_, AppState>,
    request: SyncCreate,
) -> Result<Sync, ErrorPayload> {
    state
        .syncs
        .create(request)
        .await
        .map_err(|error| ErrorPayload::from(&error))
}
#[tauri::command]
pub async fn sync_update(
    state: State<'_, AppState>,
    id: String,
    request: SyncUpdate,
) -> Result<Sync, ErrorPayload> {
    state
        .syncs
        .update(&id, request)
        .await
        .map_err(|error| ErrorPayload::from(&error))
}
#[tauri::command]
pub async fn sync_delete(
    state: State<'_, AppState>,
    id: String,
    delete_files: bool,
) -> Result<(), ErrorPayload> {
    state
        .syncs
        .delete(&id, delete_files)
        .await
        .map_err(|error| ErrorPayload::from(&error))
}
#[tauri::command]
pub async fn sync_run(state: State<'_, AppState>, id: String) -> Result<SyncResult, ErrorPayload> {
    state
        .syncs
        .run(&id)
        .await
        .map_err(|error| ErrorPayload::from(&error))
}
#[tauri::command]
pub async fn sync_items(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<SyncItem>, ErrorPayload> {
    state
        .syncs
        .items(&id)
        .await
        .map_err(|error| ErrorPayload::from(&error))
}

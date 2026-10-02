use reverb_core::queue::{DuplicateHit, EnqueueRequest, Job, MoveTarget, QueueState};
use reverb_core::CoreError;
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub async fn enqueue(
    state: State<'_, AppState>,
    request: EnqueueRequest,
) -> Result<Job, CoreError> {
    state.queue.enqueue(request).await
}

#[tauri::command]
pub async fn check_duplicates(
    state: State<'_, AppState>,
    source_ids: Vec<String>,
    profile_id: Option<String>,
    acoustid_id: Option<String>,
    mb_recording_id: Option<String>,
) -> Result<Vec<DuplicateHit>, CoreError> {
    let mut hits = state
        .queue
        .check_duplicates(source_ids.clone(), profile_id)
        .await?;
    let found = state
        .db
        .call(move |conn| {
            reverb_core::library::find_by_fingerprint(
                conn,
                acoustid_id.as_deref(),
                mb_recording_id.as_deref(),
            )
        })
        .await?;
    if !found.is_empty() {
        hits.push(DuplicateHit {
            source_id: source_ids.first().cloned().unwrap_or_default(),
            found_in: "library".into(),
            job_id: None,
        });
    }
    Ok(hits)
}

#[tauri::command]
pub async fn jobs_list(state: State<'_, AppState>) -> Result<Vec<Job>, CoreError> {
    state.queue.list().await
}

#[tauri::command]
pub async fn job_cancel(state: State<'_, AppState>, id: String) -> Result<(), CoreError> {
    state.queue.cancel(&id).await
}

#[tauri::command]
pub async fn job_retry(state: State<'_, AppState>, id: String) -> Result<Job, CoreError> {
    state.queue.retry(&id).await
}

#[tauri::command]
pub async fn job_remove(state: State<'_, AppState>, id: String) -> Result<(), CoreError> {
    state.queue.remove(&id).await
}

#[tauri::command]
pub async fn job_move(
    state: State<'_, AppState>,
    id: String,
    target: MoveTarget,
) -> Result<(), CoreError> {
    state.queue.move_job(&id, target).await
}

/// Remove `done`/`failed`/`cancelled` da lista; devolve quantos saíram.
#[tauri::command]
pub async fn jobs_clear_finished(state: State<'_, AppState>) -> Result<u32, CoreError> {
    state.queue.clear_finished().await
}

#[tauri::command]
pub async fn queue_pause(state: State<'_, AppState>) -> Result<(), CoreError> {
    state.queue.pause().await;
    Ok(())
}

#[tauri::command]
pub async fn queue_resume(state: State<'_, AppState>) -> Result<(), CoreError> {
    state.queue.resume().await;
    Ok(())
}

#[tauri::command]
pub async fn queue_state(state: State<'_, AppState>) -> Result<QueueState, CoreError> {
    state.queue.state().await
}

#[tauri::command]
pub async fn jobs_cancel_all(state: State<'_, AppState>) -> Result<(), CoreError> {
    state.queue.cancel_all().await
}

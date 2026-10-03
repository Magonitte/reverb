use crate::state::AppState;
use reverb_core::{
    artists::{ArtistHit, ArtistOptions, ArtistRelease, FollowedArtist},
    import::{ImportAnalysis, ImportSelection},
    queue::Job,
    CoreError,
};
use tauri::State;
#[tauri::command]
pub async fn import_analyze(
    state: State<'_, AppState>,
    url: String,
) -> Result<ImportAnalysis, CoreError> {
    state.imports.analyze(&url).await
}
#[tauri::command]
pub async fn import_enqueue(
    state: State<'_, AppState>,
    selection: ImportSelection,
) -> Result<Vec<Job>, CoreError> {
    state.imports.enqueue(selection, &state.syncs).await
}
#[tauri::command]
pub async fn artists_search(
    state: State<'_, AppState>,
    name: String,
) -> Result<Vec<ArtistHit>, CoreError> {
    state.artists.search(&name).await
}
#[tauri::command]
pub async fn artist_follow(
    state: State<'_, AppState>,
    provider_artist_id: String,
    options: ArtistOptions,
) -> Result<FollowedArtist, CoreError> {
    state.artists.follow(&provider_artist_id, options).await
}
#[tauri::command]
pub async fn artist_update(
    state: State<'_, AppState>,
    id: String,
    options: ArtistOptions,
) -> Result<(), CoreError> {
    state.artists.update(&id, options).await
}
#[tauri::command]
pub async fn artist_unfollow(
    state: State<'_, AppState>,
    id: String,
    delete_files: bool,
) -> Result<(), CoreError> {
    state.artists.unfollow(&id, delete_files).await
}
#[tauri::command]
pub async fn artists_followed(
    state: State<'_, AppState>,
) -> Result<Vec<FollowedArtist>, CoreError> {
    state.artists.followed().await
}
#[tauri::command]
pub async fn artist_releases(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<ArtistRelease>, CoreError> {
    state.artists.releases(Some(id)).await
}
#[tauri::command]
pub async fn artists_check_now(state: State<'_, AppState>) -> Result<Vec<Job>, CoreError> {
    state.artists.check(None).await
}
#[tauri::command]
pub async fn missing_list(
    state: State<'_, AppState>,
    artist_id: Option<String>,
) -> Result<Vec<ArtistRelease>, CoreError> {
    state.artists.missing(artist_id).await
}
#[tauri::command]
pub async fn missing_download(
    state: State<'_, AppState>,
    release_ids: Vec<String>,
) -> Result<Vec<Job>, CoreError> {
    state.artists.download_missing(release_ids).await
}

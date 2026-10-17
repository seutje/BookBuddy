use super::AppState;
use crate::{
    domain::AppError,
    services::updates::{self, Check, Progress, UpdateState},
};
use tauri::{ipc::Channel, AppHandle, Runtime, State};

#[tauri::command]
pub async fn check_app_update<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, UpdateState>,
) -> Result<Check, AppError> {
    updates::check(&app, &state).await
}

#[tauri::command]
pub async fn install_app_update(
    state: State<'_, UpdateState>,
    storage: State<'_, AppState>,
    version: String,
    confirmed: bool,
    progress: Channel<Progress>,
) -> Result<(), AppError> {
    // Exclude all concurrent writes/restores until a verified installer handoff.
    let _guard = storage.operations.write().await;
    let directory = storage.data_path.as_deref().ok_or_else(AppError::startup)?;
    updates::install(
        &state,
        storage.database()?,
        directory,
        &version,
        confirmed,
        progress,
    )
    .await
}

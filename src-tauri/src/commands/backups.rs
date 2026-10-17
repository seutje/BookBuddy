use super::AppState;
use crate::{
    domain::{backups::*, AppError},
    persistence,
    services::backups,
};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
fn directory(state: &AppState) -> Result<&std::path::Path, AppError> {
    state.data_path.as_deref().ok_or_else(AppError::startup)
}

#[tauri::command]
pub async fn backup_status(state: State<'_, AppState>) -> Result<BackupStatus, AppError> {
    let _guard = state.operations.read().await;
    backups::status(
        state.database()?,
        directory(&state)?,
        state.automatic_problem.load(Ordering::SeqCst),
    )
    .await
}
#[tauri::command]
pub async fn update_backup_settings(
    state: State<'_, AppState>,
    settings: BackupSettings,
) -> Result<(), AppError> {
    let _guard = state.operations.write().await;
    persistence::backups::save_settings(state.database()?, &settings).await?;
    let result = backups::automatic(state.database()?, directory(&state)?, false).await;
    state
        .automatic_problem
        .store(result.is_err(), Ordering::SeqCst);
    result
}
#[tauri::command]
pub async fn export_library_backup(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<Manifest>, AppError> {
    let _guard = state.operations.read().await;
    let selected = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Export BookBuddy backup")
            .add_filter("BookBuddy backup", &["bookbuddy"])
            .set_file_name(format!("BookBuddy-{}.bookbuddy", uuid::Uuid::new_v4()))
            .blocking_save_file()
    })
    .await
    .map_err(|_| AppError::validation("The backup file picker could not finish."))?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let destination = selected
        .into_path()
        .map_err(|_| AppError::validation("Choose a regular local backup destination."))?;
    backups::export(state.database()?, directory(&state)?, &destination)
        .await
        .map(Some)
}
#[tauri::command]
pub async fn choose_library_backup(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<RestorePreview>, AppError> {
    let _guard = state.operations.read().await;
    let selected = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title("Choose BookBuddy backup")
            .add_filter("BookBuddy backup", &["bookbuddy"])
            .blocking_pick_file()
    })
    .await
    .map_err(|_| AppError::validation("The backup file picker could not finish."))?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected
        .into_path()
        .map_err(|_| AppError::validation("Choose a regular local backup file."))?
        .canonicalize()
        .map_err(|_| AppError::validation("The backup file is unavailable."))?;
    let selected = backups::inspect(&path, directory(&state)?).await?;
    let token = uuid::Uuid::new_v4().to_string();
    let preview = backups::preview(state.database()?, &selected, &token).await?;
    *state.restore_selection.lock().await = Some((token, selected));
    Ok(Some(preview))
}
#[tauri::command]
pub async fn refresh_restore_preview(
    state: State<'_, AppState>,
    token: String,
) -> Result<RestorePreview, AppError> {
    let _guard = state.operations.read().await;
    let selected = state.restore_selection.lock().await;
    let (_, selected) = selected
        .as_ref()
        .filter(|(saved, _)| saved == &token)
        .ok_or_else(|| AppError::validation("Choose the backup again."))?;
    backups::preview(state.database()?, selected, &token).await
}
#[tauri::command]
pub async fn confirm_library_restore(
    state: State<'_, AppState>,
    token: String,
    review_hash: String,
    confirm_replace: bool,
) -> Result<String, AppError> {
    if !confirm_replace {
        return Err(AppError::validation(
            "Explicitly confirm replacing the entire library before restoring.",
        ));
    }
    let _guard = state.operations.write().await;
    let selected = state.restore_selection.lock().await;
    let (_, selected) = selected
        .as_ref()
        .filter(|(saved, _)| saved == &token)
        .ok_or_else(|| AppError::validation("Choose the backup again."))?;
    let recovery = backups::restore(
        state.database()?,
        directory(&state)?,
        selected,
        &review_hash,
    )
    .await?;
    state.restored.store(true, Ordering::SeqCst);
    state
        .workbooks
        .lock()
        .map_err(|_| AppError::validation("Restore completed. Close and reopen BookBuddy."))?
        .clear();
    Ok(recovery)
}
#[tauri::command]
pub fn close_after_restore(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    if !state.restored.load(Ordering::SeqCst) {
        return Err(AppError::validation(
            "No completed restore is waiting for restart.",
        ));
    }
    app.exit(0);
    Ok(())
}

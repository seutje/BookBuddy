//! Narrow typed IPC adapters. No generic SQL or filesystem commands.
pub mod backups;
pub mod books;
pub mod custom_fields;
pub mod import;
pub mod readings;
use crate::{
    domain::{AppError, Household},
    services::household,
};
use serde::Serialize;
use sqlx::SqlitePool;
use tauri::State;

pub struct AppState {
    pub operations: tokio::sync::RwLock<()>,
    pub restored: std::sync::atomic::AtomicBool,
    pub automatic_problem: std::sync::atomic::AtomicBool,
    pub restore_selection:
        tokio::sync::Mutex<Option<(String, crate::services::backups::SelectedBackup)>>,
    pub directory_lock: Option<std::fs::File>,
    pub pool: Option<SqlitePool>,
    pub data_directory: Option<String>,
    pub data_path: Option<std::path::PathBuf>,
    pub workbooks: std::sync::Mutex<std::collections::HashMap<String, import::SelectedWorkbook>>,
}

impl AppState {
    fn database(&self) -> Result<&SqlitePool, AppError> {
        if self.restored.load(std::sync::atomic::Ordering::SeqCst)
            || self.pool.as_ref().is_some_and(|pool| pool.is_closed())
        {
            return Err(AppError::validation(
                "Restore completed. Close and reopen BookBuddy to use the restored library.",
            ));
        }
        self.pool.as_ref().ok_or_else(AppError::startup)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupStatus {
    ready: bool,
    error: Option<AppError>,
    data_directory: Option<String>,
}

#[tauri::command]
pub fn startup_status(state: State<'_, AppState>) -> StartupStatus {
    StartupStatus {
        ready: state.pool.is_some(),
        error: state.pool.is_none().then(AppError::startup),
        data_directory: state.data_directory.clone(),
    }
}

#[tauri::command]
pub async fn list_people(state: State<'_, AppState>) -> Result<Household, AppError> {
    let _guard = state.operations.read().await;
    household::list(state.database()?).await
}

#[tauri::command]
pub async fn create_person(state: State<'_, AppState>, name: String) -> Result<(), AppError> {
    let _guard = state.operations.read().await;
    household::create(state.database()?, &name).await
}

#[tauri::command]
pub async fn rename_person(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<(), AppError> {
    let _guard = state.operations.read().await;
    household::rename(state.database()?, &id, &name).await
}

#[tauri::command]
pub async fn archive_person(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let _guard = state.operations.read().await;
    household::archive(state.database()?, &id).await
}

#[tauri::command]
pub async fn select_person(state: State<'_, AppState>, id: Option<String>) -> Result<(), AppError> {
    let _guard = state.operations.read().await;
    household::select(state.database()?, id.as_deref()).await
}

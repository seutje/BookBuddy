use super::AppState;
use crate::{
    domain::{readings::*, AppError},
    services::readings,
};
use tauri::State;
#[tauri::command]
pub fn local_today_date() -> Result<String, AppError> {
    local_today()
}
#[tauri::command]
pub async fn list_readings(
    state: State<'_, AppState>,
    filter: ReadingFilter,
) -> Result<Vec<ReadingEvent>, AppError> {
    let _guard = state.operations.read().await;
    readings::list(state.database()?, filter).await
}
#[tauri::command]
pub async fn log_reading(
    state: State<'_, AppState>,
    draft: ReadingDraft,
) -> Result<String, AppError> {
    let _guard = state.operations.read().await;
    readings::log(state.database()?, draft).await
}
#[tauri::command]
pub async fn log_reading_today(
    state: State<'_, AppState>,
    book_id: String,
    participant_ids: Vec<String>,
) -> Result<String, AppError> {
    let _guard = state.operations.read().await;
    readings::log_today(state.database()?, book_id, participant_ids).await
}
#[tauri::command]
pub async fn edit_reading(
    state: State<'_, AppState>,
    id: String,
    draft: ReadingDraft,
) -> Result<(), AppError> {
    let _guard = state.operations.read().await;
    readings::edit(state.database()?, &id, draft).await
}
#[tauri::command]
pub async fn delete_reading(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let _guard = state.operations.read().await;
    readings::delete(state.database()?, &id).await
}

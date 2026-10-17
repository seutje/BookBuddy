use super::AppState;
use crate::{
    domain::{custom_fields::*, AppError},
    services::custom_fields,
};
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub async fn list_custom_fields(
    state: State<'_, AppState>,
) -> Result<Vec<FieldDefinition>, AppError> {
    custom_fields::list(state.database()?).await
}
#[tauri::command]
pub async fn create_custom_field(
    state: State<'_, AppState>,
    draft: FieldDraft,
) -> Result<String, AppError> {
    custom_fields::create(state.database()?, draft).await
}
#[tauri::command]
pub async fn preview_custom_field_change(
    state: State<'_, AppState>,
    id: String,
    draft: FieldDraft,
) -> Result<ChangePreview, AppError> {
    custom_fields::preview_change(state.database()?, &id, draft).await
}
#[tauri::command]
pub async fn update_custom_field(
    state: State<'_, AppState>,
    id: String,
    draft: FieldDraft,
    token: String,
    confirmed: bool,
) -> Result<(), AppError> {
    custom_fields::update(state.database()?, &id, draft, &token, confirmed).await
}
#[tauri::command]
pub async fn reorder_custom_fields(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<(), AppError> {
    custom_fields::reorder(state.database()?, ids).await
}
#[tauri::command]
pub async fn get_custom_values(
    state: State<'_, AppState>,
    target: ValueTarget,
) -> Result<Vec<FieldValue>, AppError> {
    custom_fields::values(state.database()?, target).await
}
#[tauri::command]
pub async fn set_custom_value(
    state: State<'_, AppState>,
    target: ValueTarget,
    field_id: String,
    value: Option<Value>,
) -> Result<(), AppError> {
    custom_fields::set_value(state.database()?, target, &field_id, value).await
}

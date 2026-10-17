use super::AppState;
use crate::{
    domain::{books::*, AppError},
    services::books,
};
use tauri::State;

#[tauri::command]
pub async fn list_books(
    state: State<'_, AppState>,
    filter: BookFilter,
) -> Result<Vec<BookDetail>, AppError> {
    books::list(state.database()?, filter).await
}
#[tauri::command]
pub async fn get_book(state: State<'_, AppState>, id: String) -> Result<BookDetail, AppError> {
    books::get(state.database()?, &id).await
}
#[tauri::command]
pub async fn create_book(
    state: State<'_, AppState>,
    draft: BookDraft,
    person_id: Option<String>,
) -> Result<String, AppError> {
    books::create(state.database()?, draft, person_id.as_deref()).await
}
#[tauri::command]
pub async fn update_book(
    state: State<'_, AppState>,
    id: String,
    draft: BookDraft,
) -> Result<(), AppError> {
    books::update(state.database()?, &id, draft).await
}
#[tauri::command]
pub async fn ensure_person_book(
    state: State<'_, AppState>,
    person_id: String,
    book_id: String,
) -> Result<(), AppError> {
    books::ensure_membership(state.database()?, &person_id, &book_id).await
}
#[tauri::command]
pub async fn update_person_book(
    state: State<'_, AppState>,
    person_id: String,
    book_id: String,
    draft: PersonalDraft,
) -> Result<(), AppError> {
    books::update_personal(state.database()?, &person_id, &book_id, draft).await
}
#[tauri::command]
pub async fn mark_read(
    state: State<'_, AppState>,
    person_id: String,
    book_id: String,
) -> Result<(), AppError> {
    books::mark_read(state.database()?, &person_id, &book_id).await
}
#[tauri::command]
pub async fn duplicate_book_hints(
    state: State<'_, AppState>,
    draft: BookDraft,
    exclude_id: Option<String>,
) -> Result<Vec<Book>, AppError> {
    books::duplicate_hints(state.database()?, draft, exclude_id.as_deref()).await
}

use super::AppState;
use crate::{
    domain::{import::*, AppError},
    services::{import_commit, import_history, import_preview, xlsx_reader},
};
use std::path::PathBuf;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Clone)]
pub struct SelectedWorkbook {
    pub path: PathBuf,
    pub data: WorkbookData,
}

#[tauri::command]
pub async fn choose_library_xlsx(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<ImportPreview>, AppError> {
    let _guard = state.operations.read().await;
    let selected = tauri::async_runtime::spawn_blocking(
        move || -> Result<Option<SelectedWorkbook>, AppError> {
            let Some(file) = app
                .dialog()
                .file()
                .set_title("Choose Excel library")
                .add_filter("Excel workbook", &["xlsx"])
                .blocking_pick_file()
            else {
                return Ok(None);
            };
            let path = file
                .into_path()
                .map_err(|_| AppError::validation("Choose a regular local file."))?
                .canonicalize()
                .map_err(|_| AppError::validation("The selected file is unavailable."))?;
            let data = xlsx_reader::inspect(&path)?;
            Ok(Some(SelectedWorkbook { path, data }))
        },
    )
    .await
    .map_err(|_| {
        AppError::validation("Workbook inspection could not finish. Choose the file again.")
    })??;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let token = uuid::Uuid::new_v4().to_string();
    let draft =
        import_preview::initial_draft(state.database()?, &selected.data.source_name, &token)
            .await?;
    let preview = import_preview::preview(state.database()?, &selected.data, draft).await?;
    let mut registry = state
        .workbooks
        .lock()
        .map_err(|_| AppError::validation("Choose the workbook again."))?;
    registry.clear();
    registry.insert(token, selected);
    Ok(Some(preview))
}

pub fn selection(state: &AppState, token: &str) -> Result<SelectedWorkbook, AppError> {
    state
        .workbooks
        .lock()
        .map_err(|_| AppError::validation("Choose the workbook again."))?
        .get(token)
        .cloned()
        .ok_or_else(|| {
            AppError::validation("The workbook selection expired. Choose the file again.")
        })
}

#[tauri::command]
pub async fn preview_library_xlsx_mapping(
    state: State<'_, AppState>,
    draft: ImportReviewDraft,
) -> Result<ImportPreview, AppError> {
    let _guard = state.operations.read().await;
    let selected = selection(&state, &draft.selection_token)?;
    import_preview::preview(state.database()?, &selected.data, draft).await
}

#[tauri::command]
pub async fn commit_library_xlsx_import(
    state: State<'_, AppState>,
    draft: ImportReviewDraft,
    review_hash: String,
) -> Result<ImportSummary, AppError> {
    let _guard = state.operations.read().await;
    let selected = selection(&state, &draft.selection_token)?;
    let directory = state
        .data_path
        .as_deref()
        .ok_or_else(|| AppError::validation("The local data directory is unavailable."))?;
    import_commit::commit(
        state.database()?,
        directory,
        &selected.path,
        &selected.data,
        draft,
        &review_hash,
    )
    .await
}
#[tauri::command]
pub async fn list_import_batches(
    state: State<'_, AppState>,
) -> Result<Vec<ImportBatchInfo>, AppError> {
    let _guard = state.operations.read().await;
    import_history::batches(state.database()?).await
}
#[tauri::command]
pub async fn list_import_rows(
    state: State<'_, AppState>,
    batch_id: String,
) -> Result<Vec<ImportedRowInfo>, AppError> {
    let _guard = state.operations.read().await;
    import_history::rows(state.database()?, &batch_id).await
}
#[tauri::command]
pub async fn list_title_verification_tasks(
    state: State<'_, AppState>,
    include_resolved: bool,
) -> Result<Vec<TitleTask>, AppError> {
    let _guard = state.operations.read().await;
    import_history::tasks(state.database()?, include_resolved).await
}
#[tauri::command]
pub async fn resolve_title_verification(
    state: State<'_, AppState>,
    id: String,
    resolution: TitleResolution,
) -> Result<(), AppError> {
    let _guard = state.operations.read().await;
    import_history::resolve_title(state.database()?, &id, resolution).await
}
#[tauri::command]
pub async fn list_historical_loans(
    state: State<'_, AppState>,
    book_id: Option<String>,
) -> Result<Vec<HistoricalLoan>, AppError> {
    let _guard = state.operations.read().await;
    import_history::loans(state.database()?, book_id.as_deref()).await
}
#[tauri::command]
pub async fn get_borrow_summary(
    state: State<'_, AppState>,
    book_id: String,
) -> Result<Option<BorrowSummary>, AppError> {
    let _guard = state.operations.read().await;
    import_history::baseline(state.database()?, &book_id).await
}
#[tauri::command]
pub async fn link_historical_loan(
    state: State<'_, AppState>,
    id: String,
    book_id: String,
    expected_book_id: Option<String>,
) -> Result<(), AppError> {
    let _guard = state.operations.read().await;
    import_history::link_loan(
        state.database()?,
        &id,
        &book_id,
        expected_book_id.as_deref(),
    )
    .await
}

use super::AppState;
use crate::{
    domain::{import::*, AppError},
    services::{import_preview, xlsx_reader},
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
    let draft = ImportReviewDraft::initial(&token);
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
    let selected = selection(&state, &draft.selection_token)?;
    import_preview::preview(state.database()?, &selected.data, draft).await
}

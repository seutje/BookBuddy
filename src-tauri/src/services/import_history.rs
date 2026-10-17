use crate::{
    domain::{self, books::BookDraft, import::*, AppError},
    persistence::{books, import as repository},
};
use sqlx::SqlitePool;

pub async fn batches(pool: &SqlitePool) -> Result<Vec<ImportBatchInfo>, AppError> {
    repository::batches(pool).await
}
pub async fn rows(pool: &SqlitePool, batch: &str) -> Result<Vec<ImportedRowInfo>, AppError> {
    domain::durable_id(batch)?;
    repository::rows(pool, batch).await
}
pub async fn tasks(pool: &SqlitePool, include_resolved: bool) -> Result<Vec<TitleTask>, AppError> {
    repository::tasks(pool, include_resolved).await
}
pub async fn loans(
    pool: &SqlitePool,
    book_id: Option<&str>,
) -> Result<Vec<HistoricalLoan>, AppError> {
    if let Some(book_id) = book_id {
        domain::durable_id(book_id)?;
    }
    repository::historical_loans(pool, book_id).await
}
pub async fn baseline(pool: &SqlitePool, book_id: &str) -> Result<Option<BorrowSummary>, AppError> {
    domain::durable_id(book_id)?;
    repository::borrow_summary(pool, book_id).await
}

pub async fn resolve_title(
    pool: &SqlitePool,
    id: &str,
    resolution: TitleResolution,
) -> Result<(), AppError> {
    domain::durable_id(id)?;
    if let Some(book_id) = &resolution.book_id {
        domain::durable_id(book_id)?;
    }
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let task = repository::task_in(&mut transaction, id).await?;
    if task.state != resolution.expected_state {
        return Err(AppError::validation(
            "This title task changed after review. Refresh the queue and review it again.",
        ));
    }
    if task.book_id.is_some() && task.book_id != resolution.book_id {
        return Err(AppError::validation("This task is already linked to a book. Review its current shared title; unrelated books are not merged or reassigned."));
    }
    if let Some(book_id) = &resolution.book_id {
        let current = books::book_in(&mut transaction, book_id).await?;
        if resolution.expected_current_title.as_deref() != Some(&current.data.title) {
            return Err(AppError::validation(
                "The shared title changed after review. Refresh and review it again.",
            ));
        }
        if let Some(title) = &resolution.confirmed_title {
            let title = BookDraft {
                title: title.clone(),
                ..Default::default()
            }
            .validate()?
            .title;
            let mut updated = current.data;
            updated.title = title;
            books::update(&mut transaction, book_id, &updated.validate()?).await?;
        }
    } else if resolution.confirmed_title.is_some() {
        return Err(AppError::validation(
            "Choose a shared book before applying a title correction.",
        ));
    }
    repository::resolve_task(
        &mut transaction,
        id,
        resolution.book_id.as_deref(),
        resolution.confirmed_title.as_deref(),
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}
pub async fn link_loan(
    pool: &SqlitePool,
    id: &str,
    book_id: &str,
    expected_book_id: Option<&str>,
) -> Result<(), AppError> {
    domain::durable_id(id)?;
    domain::durable_id(book_id)?;
    if let Some(expected) = expected_book_id {
        domain::durable_id(expected)?;
    }
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    repository::link_loan(&mut transaction, id, book_id, expected_book_id).await?;
    transaction.commit().await?;
    Ok(())
}

use crate::domain::{
    books::{normalized, PersonalDraft},
    import::*,
    AppError,
};
use serde::Serialize;
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::collections::BTreeMap;

pub fn encode<T: Serialize + ?Sized>(value: &T) -> Result<String, AppError> {
    serde_json::to_string(value)
        .map_err(|_| AppError::validation("Import data could not be serialized safely."))
}
pub async fn sources(pool: &SqlitePool) -> Result<Vec<ImportSource>, AppError> {
    Ok(
        sqlx::query_as("SELECT * FROM import_source ORDER BY created_at, id")
            .fetch_all(pool)
            .await?,
    )
}
pub async fn batch_by_hash(
    pool: &SqlitePool,
    hash: &str,
) -> Result<Option<ImportBatchInfo>, AppError> {
    Ok(sqlx::query_as("SELECT id, origin_id, source_filename, source_hash, mapping_version, imported_at, summary_json FROM import_batch WHERE source_hash = ?").bind(hash).fetch_optional(pool).await?)
}
pub async fn batches(pool: &SqlitePool) -> Result<Vec<ImportBatchInfo>, AppError> {
    Ok(sqlx::query_as("SELECT id, origin_id, source_filename, source_hash, mapping_version, imported_at, summary_json FROM import_batch ORDER BY imported_at DESC, id").fetch_all(pool).await?)
}
pub async fn field_mappings(
    pool: &SqlitePool,
    origin: &str,
) -> Result<BTreeMap<String, String>, AppError> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT source_key, field_id FROM import_field_mapping WHERE origin_id = ?")
            .bind(origin)
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().collect())
}
pub async fn save_source(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
    filename: &str,
    carly: &str,
    mom: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO import_source (id, label, last_filename, carly_person_id, mom_person_id) VALUES (?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET last_filename = excluded.last_filename, carly_person_id = excluded.carly_person_id, mom_person_id = excluded.mom_person_id").bind(id).bind(filename).bind(filename).bind(carly).bind(mom).execute(&mut **transaction).await?;
    Ok(())
}
pub async fn insert_batch(
    transaction: &mut Transaction<'_, Sqlite>,
    summary: &ImportSummary,
    preview: &ImportPreview,
    references: &serde_json::Value,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO import_batch (id, origin_id, source_filename, source_hash, mapping_version, summary_json, review_json, warnings_json, reference_json) VALUES (?, ?, ?, ?, 1, ?, ?, ?, ?)")
        .bind(&summary.batch_id).bind(&summary.origin_id).bind(&preview.source_name).bind(&preview.source_hash).bind(encode(summary)?).bind(encode(&preview.draft)?).bind(encode(&preview.warnings)?).bind(encode(references)?).execute(&mut **transaction).await?;
    Ok(())
}
pub async fn finish_batch(
    transaction: &mut Transaction<'_, Sqlite>,
    summary: &ImportSummary,
) -> Result<(), AppError> {
    sqlx::query("UPDATE import_batch SET summary_json = ? WHERE id = ?")
        .bind(encode(summary)?)
        .bind(&summary.batch_id)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}
pub async fn insert_row(
    transaction: &mut Transaction<'_, Sqlite>,
    batch: &str,
    row: &SourceRow,
    warnings: &[ImportWarning],
) -> Result<String, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO import_source_row (id, batch_id, sheet_name, sheet_row_number, source_row_key, row_fingerprint, occurrence, raw_json, warning_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&id).bind(batch).bind(&row.sheet).bind(row.row_number as i64).bind(&row.key).bind(&row.fingerprint).bind(row.occurrence as i64).bind(encode(&row.raw)?).bind(encode(warnings)?).execute(&mut **transaction).await?;
    Ok(id)
}
pub async fn link_row_book(
    transaction: &mut Transaction<'_, Sqlite>,
    row: &str,
    book: &str,
) -> Result<(), AppError> {
    sqlx::query("UPDATE import_source_row SET book_id = ? WHERE id = ?")
        .bind(book)
        .bind(row)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}
pub async fn link_row_loan(
    transaction: &mut Transaction<'_, Sqlite>,
    row: &str,
    loan: &str,
) -> Result<(), AppError> {
    sqlx::query("UPDATE import_source_row SET loan_id = ? WHERE id = ?")
        .bind(loan)
        .bind(row)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}
pub async fn link_row_task(
    transaction: &mut Transaction<'_, Sqlite>,
    row: &str,
    task: &str,
) -> Result<(), AppError> {
    sqlx::query("UPDATE import_source_row SET verification_id = ? WHERE id = ?")
        .bind(task)
        .bind(row)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}
pub async fn rows(pool: &SqlitePool, batch: &str) -> Result<Vec<ImportedRowInfo>, AppError> {
    Ok(sqlx::query_as("SELECT id, sheet_name, sheet_row_number, raw_json, warning_json, book_id, loan_id, verification_id FROM import_source_row WHERE batch_id = ? ORDER BY sheet_name, sheet_row_number").bind(batch).fetch_all(pool).await?)
}

pub async fn membership(
    transaction: &mut Transaction<'_, Sqlite>,
    person: &str,
    book: &str,
) -> Result<Option<PersonalDraft>, AppError> {
    Ok(sqlx::query_as("SELECT reading_status, rating, list_status, notes FROM person_book WHERE person_id = ? AND book_id = ?").bind(person).bind(book).fetch_optional(&mut **transaction).await?)
}
pub async fn membership_id(
    transaction: &mut Transaction<'_, Sqlite>,
    person: &str,
    book: &str,
) -> Result<String, AppError> {
    sqlx::query_scalar("SELECT id FROM person_book WHERE person_id = ? AND book_id = ?")
        .bind(person)
        .bind(book)
        .fetch_optional(&mut **transaction)
        .await?
        .ok_or_else(|| AppError::validation("The imported personal membership is missing."))
}
pub async fn map_field(
    transaction: &mut Transaction<'_, Sqlite>,
    origin: &str,
    source_key: &str,
    field: &str,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO import_field_mapping (origin_id, source_key, field_id) VALUES (?, ?, ?) ON CONFLICT(origin_id, source_key) DO UPDATE SET field_id = excluded.field_id").bind(origin).bind(source_key).bind(field).execute(&mut **transaction).await?;
    Ok(())
}
pub async fn option_mapping(
    transaction: &mut Transaction<'_, Sqlite>,
    origin: &str,
    source_key: &str,
    field: &str,
    label: &str,
) -> Result<Option<String>, AppError> {
    Ok(sqlx::query_scalar("SELECT option_id FROM import_option_mapping WHERE origin_id = ? AND source_key = ? AND field_id = ? AND source_label_key = ?").bind(origin).bind(source_key).bind(field).bind(normalized(label)).fetch_optional(&mut **transaction).await?)
}
pub async fn map_option(
    transaction: &mut Transaction<'_, Sqlite>,
    origin: &str,
    source_key: &str,
    field: &str,
    label: &str,
    option: &str,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO import_option_mapping (origin_id, source_key, field_id, source_label_key, option_id) VALUES (?, ?, ?, ?, ?) ON CONFLICT(origin_id, source_key, field_id, source_label_key) DO NOTHING").bind(origin).bind(source_key).bind(field).bind(normalized(label)).bind(option).execute(&mut **transaction).await?;
    Ok(())
}
pub async fn known_loan(
    transaction: &mut Transaction<'_, Sqlite>,
    origin: &str,
    key: &str,
) -> Result<Option<String>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM library_borrow_event WHERE origin_id = ? AND source_row_key = ?",
    )
    .bind(origin)
    .bind(key)
    .fetch_optional(&mut **transaction)
    .await?)
}
pub async fn insert_loan(
    transaction: &mut Transaction<'_, Sqlite>,
    origin: &str,
    row: &BorrowRow,
    source_id: &str,
    person: &str,
    book: Option<&str>,
) -> Result<String, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO library_borrow_event (id, origin_id, source_row_key, book_id, person_id, source_title, source_author, source_format, publication_year, source_date_json, date_interpretation, date_precision, source_pdf, imported_row_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&id).bind(origin).bind(&row.source.key).bind(book).bind(person).bind(&row.title).bind(&row.author).bind(&row.format).bind(row.publication_year).bind(encode(&row.source_date)?).bind(row.date_type.as_deref().unwrap_or("Unverified historical source date; possible return-date proxy")).bind(&row.source_date.precision).bind(&row.source_pdf).bind(source_id).execute(&mut **transaction).await?;
    Ok(id)
}
pub async fn save_baseline(
    transaction: &mut Transaction<'_, Sqlite>,
    origin: &str,
    book: &str,
    row: &CatalogueRow,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO book_borrow_summary (book_id, origin_id, imported_count_baseline, first_source_date_json, latest_source_date_json, source_date_type) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(book_id) DO UPDATE SET imported_count_baseline = COALESCE(excluded.imported_count_baseline, book_borrow_summary.imported_count_baseline), first_source_date_json = CASE WHEN json_extract(excluded.first_source_date_json, '$.value') IS NULL THEN book_borrow_summary.first_source_date_json ELSE excluded.first_source_date_json END, latest_source_date_json = CASE WHEN json_extract(excluded.latest_source_date_json, '$.value') IS NULL THEN book_borrow_summary.latest_source_date_json ELSE excluded.latest_source_date_json END, source_date_type = COALESCE(excluded.source_date_type, book_borrow_summary.source_date_type), updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE book_borrow_summary.origin_id = excluded.origin_id")
        .bind(book).bind(origin).bind(row.borrow_count).bind(encode(&row.first_source_date)?).bind(encode(&row.latest_source_date)?).bind(&row.date_type).execute(&mut **transaction).await?;
    Ok(())
}
const TASK_PROJECTION: &str = "SELECT t.id, t.book_id, b.title AS current_title, t.working_title, t.raw_pdf_title, t.reason, t.proposed_confirmed_title, t.state, t.source_row_id FROM book_title_verification t LEFT JOIN book b ON b.id = t.book_id";
pub async fn task_in(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<TitleTask, AppError> {
    let mut query = sqlx::QueryBuilder::<Sqlite>::new(TASK_PROJECTION);
    query.push(" WHERE t.id = ").push_bind(id);
    query
        .build_query_as()
        .fetch_optional(&mut **transaction)
        .await?
        .ok_or_else(|| AppError::validation("The title-verification task does not exist."))
}
pub async fn known_task(
    transaction: &mut Transaction<'_, Sqlite>,
    origin: &str,
    key: &str,
) -> Result<Option<String>, AppError> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM book_title_verification WHERE origin_id = ? AND source_row_key = ?",
    )
    .bind(origin)
    .bind(key)
    .fetch_optional(&mut **transaction)
    .await?)
}
pub async fn insert_task(
    transaction: &mut Transaction<'_, Sqlite>,
    origin: &str,
    row: &TitleReviewRow,
    source_id: &str,
    book: Option<&str>,
) -> Result<String, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO book_title_verification (id, origin_id, source_row_key, book_id, working_title, raw_pdf_title, reason, proposed_confirmed_title, source_row_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&id).bind(origin).bind(&row.identity_key).bind(book).bind(&row.working_title).bind(&row.raw_pdf_title).bind(&row.reason).bind(&row.confirmed_title).bind(source_id).execute(&mut **transaction).await?;
    Ok(id)
}
pub async fn has_task_for_title_or_book(
    transaction: &mut Transaction<'_, Sqlite>,
    origin: &str,
    book: &str,
    title: &str,
) -> Result<bool, AppError> {
    let rows: Vec<(Option<String>, String)> = sqlx::query_as(
        "SELECT book_id, working_title FROM book_title_verification WHERE origin_id = ?",
    )
    .bind(origin)
    .fetch_all(&mut **transaction)
    .await?;
    Ok(rows.iter().any(|(id, working)| {
        id.as_deref() == Some(book) || normalized(working) == normalized(title)
    }))
}
pub async fn flag_book(
    transaction: &mut Transaction<'_, Sqlite>,
    book: &str,
) -> Result<(), AppError> {
    sqlx::query("UPDATE book SET needs_title_verification = 1 WHERE id = ? AND NOT EXISTS (SELECT 1 FROM book_title_verification WHERE book_id = ? AND state != 'open')").bind(book).bind(book).execute(&mut **transaction).await?;
    Ok(())
}
pub async fn resolve_task(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
    book: Option<&str>,
    confirmed: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query("UPDATE book_title_verification SET book_id = ?, state = ?, proposed_confirmed_title = COALESCE(?, proposed_confirmed_title), resolved_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?").bind(book).bind(if confirmed.is_some() { "corrected" } else { "confirmed" }).bind(confirmed).bind(id).execute(&mut **transaction).await?;
    if let Some(book) = book {
        sqlx::query("UPDATE book SET needs_title_verification = 0 WHERE id = ?")
            .bind(book)
            .execute(&mut **transaction)
            .await?;
    }
    Ok(())
}
pub async fn unresolved_count(transaction: &mut Transaction<'_, Sqlite>) -> Result<i64, AppError> {
    Ok(
        sqlx::query_scalar("SELECT COUNT(*) FROM book_title_verification WHERE state = 'open'")
            .fetch_one(&mut **transaction)
            .await?,
    )
}
pub async fn tasks(pool: &SqlitePool, include_resolved: bool) -> Result<Vec<TitleTask>, AppError> {
    let mut query = sqlx::QueryBuilder::<Sqlite>::new(TASK_PROJECTION);
    if !include_resolved {
        query.push(" WHERE t.state = 'open'");
    }
    query.push(" ORDER BY t.working_title, t.id");
    Ok(query.build_query_as().fetch_all(pool).await?)
}
pub async fn historical_loans(
    pool: &SqlitePool,
    book: Option<&str>,
) -> Result<Vec<HistoricalLoan>, AppError> {
    Ok(sqlx::query_as("SELECT e.id, e.book_id, b.title AS current_title, e.person_id, p.display_name AS person_name, e.source_title, e.source_author, e.source_format, e.publication_year, e.source_date_json, e.date_interpretation, e.date_precision, e.source_pdf, e.imported_row_id FROM library_borrow_event e LEFT JOIN book b ON b.id = e.book_id JOIN person p ON p.id = e.person_id WHERE (? IS NULL OR e.book_id = ?) ORDER BY e.created_at, e.id").bind(book).bind(book).fetch_all(pool).await?)
}
pub async fn borrow_summary(
    pool: &SqlitePool,
    book: &str,
) -> Result<Option<BorrowSummary>, AppError> {
    Ok(sqlx::query_as("SELECT s.book_id, s.imported_count_baseline, s.first_source_date_json, s.latest_source_date_json, s.source_date_type, (SELECT COUNT(*) FROM library_borrow_event WHERE book_id = s.book_id) AS linked_history_count FROM book_borrow_summary s WHERE s.book_id = ?").bind(book).fetch_optional(pool).await?)
}
pub async fn link_loan(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
    book: &str,
    expected_book: Option<&str>,
) -> Result<(), AppError> {
    super::books::require_book(transaction, book).await?;
    let changed =
        sqlx::query("UPDATE library_borrow_event SET book_id = ? WHERE id = ? AND book_id IS ?")
            .bind(book)
            .bind(id)
            .bind(expected_book)
            .execute(&mut **transaction)
            .await?
            .rows_affected();
    if changed == 0 {
        return Err(AppError::validation("The historical loan link changed after review. Refresh Import History and review it again."));
    }
    Ok(())
}

pub async fn update_open_task_proposal(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
    row: &TitleReviewRow,
) -> Result<(), AppError> {
    sqlx::query("UPDATE book_title_verification SET proposed_confirmed_title = COALESCE(?, proposed_confirmed_title), reason = COALESCE(?, reason) WHERE id = ? AND state = 'open'").bind(&row.confirmed_title).bind(&row.reason).bind(id).execute(&mut **transaction).await?;
    Ok(())
}
pub async fn task_identities(
    pool: &SqlitePool,
    origin: &str,
) -> Result<Vec<TaskIdentity>, AppError> {
    Ok(sqlx::query_as("SELECT id, source_row_key, book_id, state, proposed_confirmed_title FROM book_title_verification WHERE origin_id = ? ORDER BY source_row_key").bind(origin).fetch_all(pool).await?)
}

pub async fn catalogue_identities(
    pool: &SqlitePool,
    origin: &str,
) -> Result<Vec<CatalogueIdentity>, AppError> {
    Ok(sqlx::query_as("SELECT source_row_key, book_id, skipped, original_title, source_title, source_author, source_row_number FROM import_catalogue_identity WHERE origin_id = ? ORDER BY source_row_key").bind(origin).fetch_all(pool).await?)
}
pub async fn map_catalogue(
    transaction: &mut Transaction<'_, Sqlite>,
    origin: &str,
    row: &CatalogueRow,
    book: Option<&str>,
    batch: &str,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO import_catalogue_identity (origin_id, source_row_key, book_id, skipped, original_title, source_title, source_author, source_row_number, batch_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(origin_id, source_row_key) DO UPDATE SET book_id = excluded.book_id, skipped = excluded.skipped, source_row_number = excluded.source_row_number, batch_id = excluded.batch_id")
        .bind(origin).bind(&row.source.key).bind(book).bind(book.is_none()).bind(&row.original_title).bind(&row.book.title).bind(&row.book.authors).bind(row.source.row_number as i64).bind(batch).execute(&mut **transaction).await?;
    Ok(())
}
pub async fn loan_identities(
    pool: &SqlitePool,
    origin: &str,
) -> Result<Vec<LoanIdentity>, AppError> {
    Ok(sqlx::query_as("SELECT id, source_row_key, book_id FROM library_borrow_event WHERE origin_id = ? ORDER BY source_row_key").bind(origin).fetch_all(pool).await?)
}

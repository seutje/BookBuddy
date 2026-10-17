use crate::domain::{readings::*, AppError};
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::collections::HashMap;

pub async fn list(
    pool: &SqlitePool,
    filter: &ReadingFilter,
) -> Result<Vec<ReadingEvent>, AppError> {
    let mut transaction = pool.begin().await?;
    let mut events = sqlx::query_as::<_, ReadingEvent>("SELECT e.*, b.title FROM reading_event e JOIN book b ON b.id = e.book_id WHERE (? IS NULL OR e.book_id = ?) AND (? IS NULL OR e.occurred_on >= ?) AND (? IS NULL OR e.occurred_on <= ?) AND (? IS NULL OR EXISTS (SELECT 1 FROM reading_event_participant ep WHERE ep.event_id = e.id AND ep.person_id = ?)) ORDER BY e.occurred_on DESC, e.created_at DESC, e.id")
        .bind(&filter.book_id).bind(&filter.book_id).bind(&filter.from).bind(&filter.from).bind(&filter.to).bind(&filter.to).bind(&filter.person_id).bind(&filter.person_id).fetch_all(&mut *transaction).await?;
    let links: Vec<(String, String, String, Option<String>)> = sqlx::query_as("SELECT ep.event_id, p.id, p.display_name, p.archived_at FROM reading_event_participant ep JOIN person p ON p.id = ep.person_id ORDER BY p.created_at, p.id").fetch_all(&mut *transaction).await?;
    let mut grouped: HashMap<String, Vec<Participant>> = HashMap::new();
    for (event_id, person_id, display_name, archived_at) in links {
        grouped.entry(event_id).or_default().push(Participant {
            person_id,
            display_name,
            archived_at,
        });
    }
    for event in &mut events {
        event.participants = grouped.remove(&event.id).unwrap_or_default();
    }
    transaction.commit().await?;
    Ok(events)
}
pub async fn participants(
    transaction: &mut Transaction<'_, Sqlite>,
    event_id: &str,
) -> Result<Vec<Participant>, AppError> {
    Ok(sqlx::query_as::<_, Participant>("SELECT p.id AS person_id, p.display_name, p.archived_at FROM person p JOIN reading_event_participant ep ON ep.person_id = p.id WHERE ep.event_id = ? ORDER BY p.created_at, p.id").bind(event_id).fetch_all(&mut **transaction).await?)
}
pub async fn get_in(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<ReadingEvent, AppError> {
    let mut event = sqlx::query_as::<_, ReadingEvent>(
        "SELECT e.*, b.title FROM reading_event e JOIN book b ON b.id = e.book_id WHERE e.id = ?",
    )
    .bind(id)
    .fetch_optional(&mut **transaction)
    .await?
    .ok_or_else(|| AppError::validation("That reading could not be found. Refresh Activity."))?;
    event.participants = participants(transaction, id).await?;
    Ok(event)
}
pub async fn person_in(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<Participant, AppError> {
    sqlx::query_as::<_, Participant>(
        "SELECT id AS person_id, display_name, archived_at FROM person WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&mut **transaction)
    .await?
    .ok_or_else(AppError::not_found)
}
pub async fn create(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
    draft: &ReadingDraft,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO reading_event (id, book_id, occurred_on, note) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(&draft.book_id)
        .bind(&draft.occurred_on)
        .bind(&draft.note)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}
pub async fn update(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
    draft: &ReadingDraft,
) -> Result<(), AppError> {
    sqlx::query("UPDATE reading_event SET occurred_on = ?, note = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?").bind(&draft.occurred_on).bind(&draft.note).bind(id).execute(&mut **transaction).await?;
    sqlx::query("DELETE FROM reading_event_participant WHERE event_id = ?")
        .bind(id)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}
pub async fn add_participant(
    transaction: &mut Transaction<'_, Sqlite>,
    event_id: &str,
    person_id: &str,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO reading_event_participant (event_id, person_id) VALUES (?, ?)")
        .bind(event_id)
        .bind(person_id)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}
pub async fn delete(transaction: &mut Transaction<'_, Sqlite>, id: &str) -> Result<(), AppError> {
    get_in(transaction, id).await?;
    sqlx::query("DELETE FROM reading_event WHERE id = ?")
        .bind(id)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

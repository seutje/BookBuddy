use crate::{
    domain::{durable_id, readings::*, AppError},
    persistence::{books, household, readings as repository},
};
use sqlx::{Sqlite, SqlitePool, Transaction};

pub async fn list(pool: &SqlitePool, filter: ReadingFilter) -> Result<Vec<ReadingEvent>, AppError> {
    filter.validate()?;
    repository::list(pool, &filter).await
}
async fn save_participants(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
    draft: &ReadingDraft,
    previous: &[Participant],
) -> Result<(), AppError> {
    for person_id in &draft.participant_ids {
        let person = repository::person_in(transaction, person_id).await?;
        if person.archived_at.is_some() {
            if !previous
                .iter()
                .any(|participant| &participant.person_id == person_id)
            {
                return Err(AppError::validation("Archived people can be retained in their existing readings, but cannot be added to a new reading."));
            }
        } else {
            books::ensure_membership(transaction, person_id, &draft.book_id).await?;
            if draft.set_read {
                books::mark_read(transaction, person_id, &draft.book_id).await?;
            }
        }
        repository::add_participant(transaction, id, person_id).await?;
    }
    Ok(())
}
pub async fn log(pool: &SqlitePool, draft: ReadingDraft) -> Result<String, AppError> {
    draft.validate(&local_today()?)?;
    let id = uuid::Uuid::new_v4().to_string();
    let mut transaction = household::begin(pool).await?;
    books::require_book(&mut transaction, &draft.book_id).await?;
    repository::create(&mut transaction, &id, &draft).await?;
    save_participants(&mut transaction, &id, &draft, &[]).await?;
    transaction.commit().await?;
    Ok(id)
}
pub async fn log_today(
    pool: &SqlitePool,
    book_id: String,
    participant_ids: Vec<String>,
) -> Result<String, AppError> {
    log(
        pool,
        ReadingDraft {
            book_id,
            participant_ids,
            occurred_on: local_today()?,
            note: None,
            set_read: true,
        },
    )
    .await
}
pub async fn edit(pool: &SqlitePool, id: &str, draft: ReadingDraft) -> Result<(), AppError> {
    durable_id(id)?;
    draft.validate(&local_today()?)?;
    let mut transaction = household::begin(pool).await?;
    let old = repository::get_in(&mut transaction, id).await?;
    if old.book_id != draft.book_id {
        return Err(AppError::validation("A reading's book cannot change. Delete the mistaken reading and log it on the correct book."));
    }
    repository::update(&mut transaction, id, &draft).await?;
    save_participants(&mut transaction, id, &draft, &old.participants).await?;
    transaction.commit().await?;
    Ok(())
}
pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    durable_id(id)?;
    let mut transaction = household::begin(pool).await?;
    repository::delete(&mut transaction, id).await?;
    transaction.commit().await?;
    Ok(())
}

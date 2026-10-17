use crate::domain::custom_fields::{canonical_value, resolve_filters, CustomMatch, Scope};
use crate::domain::{books::*, AppError};
use sqlx::{QueryBuilder, Sqlite, SqlitePool, Transaction};
use std::collections::HashMap;

const BOOK_PROJECTION: &str = "SELECT b.*, (SELECT COUNT(*) FROM reading_event e WHERE e.book_id = b.id) AS read_count, (SELECT MAX(e.occurred_on) FROM reading_event e WHERE e.book_id = b.id) AS last_read_on FROM book b";
const MEMBERSHIP_PROJECTION: &str = "SELECT pb.*, p.display_name AS person_name, p.archived_at, (SELECT COUNT(*) FROM reading_event e JOIN reading_event_participant ep ON ep.event_id = e.id WHERE e.book_id = pb.book_id AND ep.person_id = pb.person_id) AS read_count, (SELECT MAX(e.occurred_on) FROM reading_event e JOIN reading_event_participant ep ON ep.event_id = e.id WHERE e.book_id = pb.book_id AND ep.person_id = pb.person_id) AS last_read_on FROM person_book pb JOIN person p ON p.id = pb.person_id WHERE (? IS NULL OR pb.book_id = ?) ORDER BY p.created_at, p.id";

pub async fn all(pool: &SqlitePool) -> Result<Vec<BookDetail>, AppError> {
    filtered(pool, &BookFilter::default()).await
}

pub async fn filtered(pool: &SqlitePool, filter: &BookFilter) -> Result<Vec<BookDetail>, AppError> {
    let mut transaction = pool.begin().await?;
    let mut query = QueryBuilder::<Sqlite>::new(BOOK_PROJECTION);
    query.push(" WHERE 1 = 1");
    let definitions = super::custom_fields::definitions_in(&mut transaction).await?;
    let custom = resolve_filters(&definitions, &filter.custom_filters)?;
    for (definition, filter) in custom
        .iter()
        .filter(|(definition, _)| definition.draft.scope == Scope::Book)
    {
        match &filter.condition {
            CustomMatch::Unknown => {
                query.push(" AND NOT EXISTS (SELECT 1 FROM book_custom_field_value cv WHERE cv.book_id = b.id AND cv.field_id = ").push_bind(&definition.id).push(")");
            }
            CustomMatch::Equals(value) => {
                query.push(" AND EXISTS (SELECT 1 FROM book_custom_field_value cv WHERE cv.book_id = b.id AND cv.field_id = ").push_bind(&definition.id).push(" AND cv.value_json = ").push_bind(canonical_value(&definition.draft, value)?).push(")");
            }
        }
    }
    match filter.owned {
        Filter::Any => {}
        Filter::Unknown => {
            query.push(" AND b.owned IS NULL");
        }
        Filter::Equals(value) => {
            query.push(" AND b.owned = ").push_bind(value);
        }
    }
    if filter.person_id.is_some()
        || !matches!(filter.reading_status, Filter::Any)
        || !matches!(filter.rating, Filter::Any)
        || !matches!(filter.list_status, Filter::Any)
        || custom
            .iter()
            .any(|(definition, _)| definition.draft.scope == Scope::PersonBook)
    {
        query.push(" AND EXISTS (SELECT 1 FROM person_book pb WHERE pb.book_id = b.id");
        if let Some(person) = &filter.person_id {
            query.push(" AND pb.person_id = ").push_bind(person);
        }
        match filter.reading_status {
            Filter::Any => {}
            Filter::Unknown => {
                query.push(" AND pb.reading_status IS NULL");
            }
            Filter::Equals(value) => {
                query.push(" AND pb.reading_status = ").push_bind(value);
            }
        }
        match filter.rating {
            Filter::Any => {}
            Filter::Unknown => {
                query.push(" AND pb.rating IS NULL");
            }
            Filter::Equals(value) => {
                query.push(" AND pb.rating = ").push_bind(value);
            }
        }
        match filter.list_status {
            Filter::Any => {}
            Filter::Unknown => {
                query.push(" AND pb.list_status IS NULL");
            }
            Filter::Equals(value) => {
                query.push(" AND pb.list_status = ").push_bind(value);
            }
        }
        for (definition, filter) in custom
            .iter()
            .filter(|(definition, _)| definition.draft.scope == Scope::PersonBook)
        {
            match &filter.condition {
                CustomMatch::Unknown => {
                    query.push(" AND NOT EXISTS (SELECT 1 FROM person_book_custom_field_value cv WHERE cv.person_book_id = pb.id AND cv.field_id = ").push_bind(&definition.id).push(")");
                }
                CustomMatch::Equals(value) => {
                    query.push(" AND EXISTS (SELECT 1 FROM person_book_custom_field_value cv WHERE cv.person_book_id = pb.id AND cv.field_id = ").push_bind(&definition.id).push(" AND cv.value_json = ").push_bind(canonical_value(&definition.draft, value)?).push(")");
                }
            }
        }
        query.push(")");
    }
    query.push(" ORDER BY b.created_at DESC, b.id");
    let books = query
        .build_query_as::<Book>()
        .fetch_all(&mut *transaction)
        .await?;
    let memberships = sqlx::query_as::<_, Membership>(MEMBERSHIP_PROJECTION)
        .bind(None::<&str>)
        .bind(None::<&str>)
        .fetch_all(&mut *transaction)
        .await?;
    let mut grouped: HashMap<String, Vec<Membership>> = HashMap::new();
    for membership in memberships {
        grouped
            .entry(membership.book_id.clone())
            .or_default()
            .push(membership);
    }
    let mut rows: Vec<BookDetail> = books
        .into_iter()
        .map(|book| {
            let memberships = grouped.remove(&book.id).unwrap_or_default();
            BookDetail {
                book,
                memberships,
                row_fields: Vec::new(),
                custom_search: Vec::new(),
            }
        })
        .collect();
    super::custom_fields::decorate(&mut transaction, &mut rows, filter.person_id.as_deref())
        .await?;
    transaction.commit().await?;
    Ok(rows)
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<BookDetail, AppError> {
    let mut transaction = pool.begin().await?;
    let mut query = QueryBuilder::<Sqlite>::new(BOOK_PROJECTION);
    query.push(" WHERE b.id = ").push_bind(id);
    let book = query
        .build_query_as::<Book>()
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| {
            AppError::validation("That book could not be found. Refresh the library.")
        })?;
    let memberships = sqlx::query_as::<_, Membership>(MEMBERSHIP_PROJECTION)
        .bind(id)
        .bind(id)
        .fetch_all(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(BookDetail {
        book,
        memberships,
        row_fields: Vec::new(),
        custom_search: Vec::new(),
    })
}

pub async fn require_book(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<(), AppError> {
    let found: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM book WHERE id = ?")
        .bind(id)
        .fetch_one(&mut **transaction)
        .await?;
    if found == 0 {
        return Err(AppError::validation("That book could not be found."));
    }
    Ok(())
}

pub async fn create(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
    draft: &BookDraft,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO book (id, title, subtitle, authors, isbn13, publisher, publication_year, language, series, volume, description, notes, owned) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(id).bind(&draft.title).bind(&draft.subtitle).bind(&draft.authors).bind(&draft.isbn13).bind(&draft.publisher).bind(draft.publication_year).bind(&draft.language).bind(&draft.series).bind(&draft.volume).bind(&draft.description).bind(&draft.notes).bind(draft.owned).execute(&mut **transaction).await?;
    Ok(())
}

pub async fn update(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
    draft: &BookDraft,
) -> Result<(), AppError> {
    require_book(transaction, id).await?;
    sqlx::query("UPDATE book SET title = ?, subtitle = ?, authors = ?, isbn13 = ?, publisher = ?, publication_year = ?, language = ?, series = ?, volume = ?, description = ?, notes = ?, owned = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(&draft.title).bind(&draft.subtitle).bind(&draft.authors).bind(&draft.isbn13).bind(&draft.publisher).bind(draft.publication_year).bind(&draft.language).bind(&draft.series).bind(&draft.volume).bind(&draft.description).bind(&draft.notes).bind(draft.owned).bind(id).execute(&mut **transaction).await?;
    Ok(())
}

pub async fn ensure_membership(
    transaction: &mut Transaction<'_, Sqlite>,
    person_id: &str,
    book_id: &str,
) -> Result<(), AppError> {
    super::household::require_active(transaction, person_id).await?;
    require_book(transaction, book_id).await?;
    sqlx::query("INSERT INTO person_book (id, person_id, book_id) VALUES (?, ?, ?) ON CONFLICT(person_id, book_id) DO NOTHING")
        .bind(uuid::Uuid::new_v4().to_string()).bind(person_id).bind(book_id).execute(&mut **transaction).await?;
    Ok(())
}

pub async fn update_personal(
    transaction: &mut Transaction<'_, Sqlite>,
    person_id: &str,
    book_id: &str,
    draft: &PersonalDraft,
) -> Result<(), AppError> {
    ensure_membership(transaction, person_id, book_id).await?;
    sqlx::query("UPDATE person_book SET reading_status = ?, rating = ?, list_status = ?, notes = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE person_id = ? AND book_id = ?")
        .bind(draft.reading_status).bind(draft.rating).bind(draft.list_status).bind(&draft.notes).bind(person_id).bind(book_id).execute(&mut **transaction).await?;
    Ok(())
}

pub async fn mark_read(
    transaction: &mut Transaction<'_, Sqlite>,
    person_id: &str,
    book_id: &str,
) -> Result<(), AppError> {
    ensure_membership(transaction, person_id, book_id).await?;
    sqlx::query("UPDATE person_book SET reading_status = 'read', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE person_id = ? AND book_id = ?")
        .bind(person_id).bind(book_id).execute(&mut **transaction).await?;
    Ok(())
}

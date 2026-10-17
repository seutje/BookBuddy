use crate::{
    domain::{self, books::*, AppError},
    persistence::{books as repository, household},
};
use sqlx::SqlitePool;

pub async fn create(
    pool: &SqlitePool,
    draft: BookDraft,
    person_id: Option<&str>,
) -> Result<String, AppError> {
    let draft = draft.validate()?;
    if let Some(id) = person_id {
        domain::durable_id(id)?;
    }
    let id = uuid::Uuid::new_v4().to_string();
    let mut transaction = household::begin(pool).await?;
    repository::create(&mut transaction, &id, &draft).await?;
    if let Some(person) = person_id {
        repository::ensure_membership(&mut transaction, person, &id).await?;
    }
    transaction.commit().await?;
    Ok(id)
}

pub async fn update(pool: &SqlitePool, id: &str, draft: BookDraft) -> Result<(), AppError> {
    domain::durable_id(id)?;
    let draft = draft.validate()?;
    let mut transaction = household::begin(pool).await?;
    repository::update(&mut transaction, id, &draft).await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<BookDetail, AppError> {
    domain::durable_id(id)?;
    repository::get(pool, id).await
}

pub async fn ensure_membership(
    pool: &SqlitePool,
    person_id: &str,
    book_id: &str,
) -> Result<(), AppError> {
    domain::durable_id(person_id)?;
    domain::durable_id(book_id)?;
    let mut transaction = household::begin(pool).await?;
    repository::ensure_membership(&mut transaction, person_id, book_id).await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn update_personal(
    pool: &SqlitePool,
    person_id: &str,
    book_id: &str,
    draft: PersonalDraft,
) -> Result<(), AppError> {
    domain::durable_id(person_id)?;
    domain::durable_id(book_id)?;
    validate_notes(&draft.notes)?;
    let mut transaction = household::begin(pool).await?;
    repository::update_personal(&mut transaction, person_id, book_id, &draft).await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn mark_read(pool: &SqlitePool, person_id: &str, book_id: &str) -> Result<(), AppError> {
    domain::durable_id(person_id)?;
    domain::durable_id(book_id)?;
    let mut transaction = household::begin(pool).await?;
    repository::mark_read(&mut transaction, person_id, book_id).await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn duplicate_hints(
    pool: &SqlitePool,
    draft: BookDraft,
    exclude_id: Option<&str>,
) -> Result<Vec<Book>, AppError> {
    if draft.title.len() > 10_000
        || draft
            .authors
            .as_ref()
            .is_some_and(|value| value.len() > 20_000)
        || draft.isbn13.as_ref().is_some_and(|value| value.len() > 100)
    {
        return Err(AppError::validation("Duplicate search is too long."));
    }
    let title = normalized(&draft.title);
    let author = normalized(draft.authors.as_deref().unwrap_or(""));
    let isbn: String = draft
        .isbn13
        .as_deref()
        .unwrap_or("")
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    Ok(repository::all(pool)
        .await?
        .into_iter()
        .map(|detail| detail.book)
        .filter(|book| {
            Some(book.id.as_str()) != exclude_id
                && ((!isbn.is_empty() && book.data.isbn13.as_deref() == Some(&isbn))
                    || (!title.is_empty()
                        && normalized(&book.data.title) == title
                        && (author.is_empty()
                            || normalized(book.data.authors.as_deref().unwrap_or("")) == author)))
        })
        .collect())
}

pub async fn list(pool: &SqlitePool, filter: BookFilter) -> Result<Vec<BookDetail>, AppError> {
    if let Some(id) = &filter.person_id {
        domain::durable_id(id)?;
    }
    if filter.search.len() > 5000 {
        return Err(AppError::validation("Search must be at most 5000 bytes."));
    }
    let search = normalized(&filter.search);
    let mut rows: Vec<BookDetail> = repository::filtered(pool, &filter)
        .await?
        .into_iter()
        .filter(|detail| {
            if search.is_empty() {
                return true;
            }
            let book = &detail.book.data;
            [
                &Some(book.title.clone()),
                &book.subtitle,
                &book.authors,
                &book.isbn13,
                &book.publisher,
                &book.series,
                &book.description,
                &book.notes,
            ]
            .iter()
            .any(|value| {
                value
                    .as_deref()
                    .is_some_and(|text| normalized(text).contains(&search))
            }) || detail
                .memberships
                .iter()
                .filter(|membership| {
                    filter
                        .person_id
                        .as_ref()
                        .is_none_or(|id| id == &membership.person_id)
                })
                .any(|membership| {
                    membership
                        .notes
                        .as_ref()
                        .is_some_and(|note| normalized(note).contains(&search))
                })
        })
        .collect();
    match filter.sort {
        BookSort::RecentlyAdded => {}
        BookSort::Title => rows.sort_by_cached_key(|detail| normalized(&detail.book.data.title)),
        BookSort::Author => rows.sort_by_cached_key(|detail| {
            (
                normalized(detail.book.data.authors.as_deref().unwrap_or("")),
                normalized(&detail.book.data.title),
            )
        }),
        BookSort::Rating => rows.sort_by_key(|detail| {
            let rating = detail
                .memberships
                .iter()
                .filter(|membership| {
                    filter
                        .person_id
                        .as_ref()
                        .is_none_or(|id| id == &membership.person_id)
                })
                .filter_map(|membership| membership.rating)
                .min();
            (rating.is_none(), rating)
        }),
    }
    // Each row retains the participants for an Everyone summary. React edits by explicit person ID.
    Ok(rows)
}

use bookbuddy_lib::{
    domain::{
        books::{BookDraft, ListStatus, PersonalDraft, Rating, ReadingStatus},
        readings::*,
    },
    persistence,
    services::{books, household, readings},
};
use sqlx::SqlitePool;
async fn person(pool: &SqlitePool, name: &str) -> String {
    household::create(pool, name).await.unwrap();
    household::list(pool)
        .await
        .unwrap()
        .current_person_id
        .unwrap()
}
async fn book(pool: &SqlitePool) -> String {
    books::create(
        pool,
        BookDraft {
            title: "Repeat book".into(),
            ..Default::default()
        },
        None,
    )
    .await
    .unwrap()
}
fn draft(book: &str, people: &[&str], date: &str) -> ReadingDraft {
    ReadingDraft {
        book_id: book.into(),
        occurred_on: date.into(),
        participant_ids: people.iter().map(|id| (*id).to_string()).collect(),
        note: None,
        set_read: true,
    }
}

#[tokio::test]
async fn shared_repeats_have_one_household_event_and_independent_participant_counts() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let first = person(&pool, "First").await;
    let second = person(&pool, "Second").await;
    let book = book(&pool).await;
    books::update_personal(
        &pool,
        &first,
        &book,
        PersonalDraft {
            reading_status: Some(ReadingStatus::WantToRead),
            rating: Some(Rating::Loved),
            list_status: Some(ListStatus::Wishlist),
            notes: Some("Keep this note".into()),
        },
    )
    .await
    .unwrap();
    books::mark_read(&pool, &first, &book).await.unwrap();
    let initial = books::get(&pool, &book).await.unwrap();
    assert_eq!(initial.book.read_count, 0);
    assert_eq!(initial.book.last_read_on, None);
    let one = readings::log(&pool, draft(&book, &[&first, &second], "2000-01-01"))
        .await
        .unwrap();
    let two = readings::log(&pool, draft(&book, &[&first, &second], "2000-01-01"))
        .await
        .unwrap();
    assert_ne!(one, two);
    readings::log(&pool, draft(&book, &[&first], "2000-02-01"))
        .await
        .unwrap();
    let events = readings::list(&pool, ReadingFilter::default())
        .await
        .unwrap();
    assert_eq!(events.len(), 3);
    let detail = books::get(&pool, &book).await.unwrap();
    assert_eq!(detail.book.read_count, 3);
    assert_eq!(detail.book.last_read_on.as_deref(), Some("2000-02-01"));
    let first_member = detail
        .memberships
        .iter()
        .find(|member| member.person_id == first)
        .unwrap();
    let second_member = detail
        .memberships
        .iter()
        .find(|member| member.person_id == second)
        .unwrap();
    assert_eq!(first_member.read_count, 3);
    assert_eq!(second_member.read_count, 2);
    assert_eq!(second_member.last_read_on.as_deref(), Some("2000-01-01"));
    assert_eq!(first_member.notes.as_deref(), Some("Keep this note"));
    assert_eq!(first_member.rating, Some(Rating::Loved));
    assert_eq!(first_member.list_status, Some(ListStatus::Wishlist));
    assert_eq!(
        readings::list(
            &pool,
            ReadingFilter {
                person_id: Some(second),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .len(),
        2
    );
    assert_eq!(
        readings::list(
            &pool,
            ReadingFilter {
                from: Some("2000-02-01".into()),
                to: Some("2000-02-01".into()),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .len(),
        1
    );
    pool.close().await;
    let pool = persistence::open(directory.path()).await.unwrap();
    assert_eq!(books::get(&pool, &book).await.unwrap().book.read_count, 3);
    pool.close().await;
}

#[tokio::test]
async fn edit_and_undo_update_only_event_facts_and_preserve_manual_status() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let first = person(&pool, "First").await;
    let second = person(&pool, "Second").await;
    let book = book(&pool).await;
    let event = readings::log(&pool, draft(&book, &[&first, &second], "2000-02-01"))
        .await
        .unwrap();
    let mut edit = draft(&book, &[&first], "1999-12-31");
    edit.note = Some("Corrected date\nShared note".into());
    edit.set_read = false;
    readings::edit(&pool, &event, edit).await.unwrap();
    let rows = readings::list(&pool, ReadingFilter::default())
        .await
        .unwrap();
    assert_eq!(rows[0].participants.len(), 1);
    assert_eq!(rows[0].occurred_on, "1999-12-31");
    assert_eq!(rows[0].note.as_deref(), Some("Corrected date\nShared note"));
    let detail = books::get(&pool, &book).await.unwrap();
    let removed = detail
        .memberships
        .iter()
        .find(|member| member.person_id == second)
        .unwrap();
    assert_eq!(removed.read_count, 0);
    assert_eq!(removed.reading_status, Some(ReadingStatus::Read));
    readings::delete(&pool, &event).await.unwrap();
    assert!(readings::list(&pool, ReadingFilter::default())
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM reading_event_participant")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let detail = books::get(&pool, &book).await.unwrap();
    assert_eq!(detail.book.read_count, 0);
    assert_eq!(detail.book.last_read_on, None);
    assert!(detail
        .memberships
        .iter()
        .all(|member| member.reading_status == Some(ReadingStatus::Read)));
    assert!(readings::delete(&pool, &event).await.is_err());
    pool.close().await;
}

#[tokio::test]
async fn invalid_participants_dates_and_failed_edits_roll_back_every_write() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let person = person(&pool, "Reader").await;
    let book = book(&pool).await;
    let missing = uuid::Uuid::new_v4().to_string();
    for invalid in [
        draft(&book, &[], "2000-01-01"),
        draft(&book, &[&person, &person], "2000-01-01"),
        draft(&book, &[&person], "2001-02-29"),
        draft(&book, &[&person], "9999-12-31"),
        draft(&book, &[&person, &missing], "2000-01-01"),
    ] {
        assert!(readings::log(&pool, invalid).await.is_err());
    }
    assert!(readings::list(&pool, ReadingFilter::default())
        .await
        .unwrap()
        .is_empty());
    assert!(books::get(&pool, &book)
        .await
        .unwrap()
        .memberships
        .is_empty());
    let mut initial = draft(&book, &[&person], "2000-01-01");
    initial.set_read = false;
    let event = readings::log(&pool, initial).await.unwrap();
    assert_eq!(
        books::get(&pool, &book).await.unwrap().memberships[0].reading_status,
        None
    );
    assert!(readings::edit(
        &pool,
        &event,
        draft(&book, &[&person, &missing], "2000-02-01")
    )
    .await
    .is_err());
    let rows = readings::list(&pool, ReadingFilter::default())
        .await
        .unwrap();
    assert_eq!(rows[0].occurred_on, "2000-01-01");
    assert_eq!(rows[0].participants.len(), 1);
    assert_eq!(
        books::get(&pool, &book).await.unwrap().memberships[0].reading_status,
        None
    );
    assert!(
        sqlx::query("INSERT INTO reading_event_participant VALUES (?, ?)")
            .bind(&event)
            .bind(&person)
            .execute(&pool)
            .await
            .is_err()
    );
    pool.close().await;
}

#[tokio::test]
async fn archive_preserves_history_and_archived_participants_can_only_be_retained_or_removed() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let first = person(&pool, "First").await;
    let second = person(&pool, "Second").await;
    let book = book(&pool).await;
    let event = readings::log(&pool, draft(&book, &[&first, &second], "2000-01-01"))
        .await
        .unwrap();
    books::update_personal(
        &pool,
        &second,
        &book,
        PersonalDraft {
            reading_status: Some(ReadingStatus::Reading),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    household::archive(&pool, &second).await.unwrap();
    assert!(
        readings::log(&pool, draft(&book, &[&first, &second], "2000-02-01"))
            .await
            .is_err()
    );
    readings::edit(
        &pool,
        &event,
        draft(&book, &[&first, &second], "2000-03-01"),
    )
    .await
    .unwrap();
    let detail = books::get(&pool, &book).await.unwrap();
    assert_eq!(
        detail
            .memberships
            .iter()
            .find(|member| member.person_id == second)
            .unwrap()
            .reading_status,
        Some(ReadingStatus::Reading)
    );
    let rows = readings::list(&pool, ReadingFilter::default())
        .await
        .unwrap();
    assert!(rows[0]
        .participants
        .iter()
        .any(|participant| participant.archived_at.is_some()));
    assert!(sqlx::query("DELETE FROM person WHERE id = ?")
        .bind(&second)
        .execute(&pool)
        .await
        .is_err());
    readings::edit(&pool, &event, draft(&book, &[&first], "2000-03-01"))
        .await
        .unwrap();
    assert!(readings::edit(
        &pool,
        &event,
        draft(&book, &[&first, &second], "2000-03-01")
    )
    .await
    .is_err());
    pool.close().await;
}

#[test]
fn reading_calendar_dates_use_local_offsets_without_utc_rounding() {
    let near_midnight = time::macros::datetime!(2024-01-01 00:30 UTC);
    let west = near_midnight.to_offset(time::UtcOffset::from_hms(-1, 0, 0).unwrap());
    assert_eq!(calendar_date_at(west), "2023-12-31");
    let late = time::macros::datetime!(2024-01-01 23:30 UTC);
    let east = late.to_offset(time::UtcOffset::from_hms(2, 0, 0).unwrap());
    assert_eq!(calendar_date_at(east), "2024-01-02");
    assert!(bookbuddy_lib::domain::custom_fields::valid_date(
        &local_today().unwrap()
    ));
}

#[tokio::test]
async fn last_reading_filters_and_sorts_use_actual_events_in_the_selected_scope() {
    use bookbuddy_lib::domain::books::{BookFilter, BookSort};
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let first = person(&pool, "First").await;
    let second = person(&pool, "Second").await;
    let earlier = book(&pool).await;
    let later = book(&pool).await;
    let manual = book(&pool).await;
    for id in [&earlier, &later, &manual] {
        books::ensure_membership(&pool, &first, id).await.unwrap();
        books::ensure_membership(&pool, &second, id).await.unwrap();
    }
    books::mark_read(&pool, &first, &manual).await.unwrap();
    readings::log(&pool, draft(&earlier, &[&first], "2000-01-01"))
        .await
        .unwrap();
    readings::log(&pool, draft(&earlier, &[&first], "2000-01-01"))
        .await
        .unwrap();
    readings::log(&pool, draft(&later, &[&second], "2000-02-01"))
        .await
        .unwrap();
    let rows = books::list(
        &pool,
        BookFilter {
            sort: BookSort::MostRecentlyRead,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(rows[0].book.id, later);
    assert_eq!(rows[2].book.id, manual);
    let rows = books::list(
        &pool,
        BookFilter {
            sort: BookSort::ReadingCount,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(rows[0].book.id, earlier);
    let rows = books::list(
        &pool,
        BookFilter {
            person_id: Some(first.clone()),
            read_from: Some("2000-02-01".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(rows.is_empty());
    let rows = books::list(
        &pool,
        BookFilter {
            person_id: Some(first),
            has_readings: Some(false),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    let rows = books::list(
        &pool,
        BookFilter {
            read_from: Some("2000-02-01".into()),
            read_to: Some("2000-02-01".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].book.id, later);
    assert!(books::list(
        &pool,
        BookFilter {
            read_from: Some("invalid".into()),
            ..Default::default()
        }
    )
    .await
    .is_err());
    pool.close().await;
}

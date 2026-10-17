use bookbuddy_lib::{
    domain::{books::*, ErrorCode},
    persistence,
    services::{books, household},
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
fn title(value: &str) -> BookDraft {
    BookDraft {
        title: value.into(),
        ..Default::default()
    }
}

#[tokio::test]
async fn shared_catalog_and_personal_values_remain_independent_after_restart() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let first = person(&pool, "Reader A").await;
    let second = person(&pool, "Reader B").await;
    let book = books::create(&pool, title("  Title only  "), None)
        .await
        .unwrap();
    assert!(books::get(&pool, &book)
        .await
        .unwrap()
        .memberships
        .is_empty());
    assert!(books::list(
        &pool,
        BookFilter {
            person_id: Some(first.clone()),
            ..Default::default()
        }
    )
    .await
    .unwrap()
    .is_empty());
    for _ in 0..2 {
        books::ensure_membership(&pool, &first, &book)
            .await
            .unwrap();
    }
    books::update_personal(
        &pool,
        &first,
        &book,
        PersonalDraft {
            reading_status: Some(ReadingStatus::Read),
            rating: Some(Rating::Loved),
            list_status: Some(ListStatus::Wishlist),
            notes: Some("first person's note".into()),
        },
    )
    .await
    .unwrap();
    books::update_personal(
        &pool,
        &second,
        &book,
        PersonalDraft {
            reading_status: Some(ReadingStatus::WantToRead),
            rating: Some(Rating::Nope),
            list_status: Some(ListStatus::Library),
            notes: Some("second person's note".into()),
        },
    )
    .await
    .unwrap();
    let detail = books::get(&pool, &book).await.unwrap();
    let mut shared = detail.book.data;
    assert_eq!(shared.title, "Title only");
    assert_eq!(shared.owned, None);
    shared.owned = Some(false);
    shared.authors = Some("Shared author".into());
    books::update(&pool, &book, shared).await.unwrap();
    books::mark_read(&pool, &second, &book).await.unwrap();
    pool.close().await;
    let pool = persistence::open(directory.path()).await.unwrap();
    let detail = books::get(&pool, &book).await.unwrap();
    assert_eq!(detail.book.data.owned, Some(false));
    assert_eq!(detail.memberships.len(), 2);
    let a = detail
        .memberships
        .iter()
        .find(|m| m.person_id == first)
        .unwrap();
    let b = detail
        .memberships
        .iter()
        .find(|m| m.person_id == second)
        .unwrap();
    assert_eq!(a.rating, Some(Rating::Loved));
    assert_eq!(a.list_status, Some(ListStatus::Wishlist));
    assert_eq!(a.notes.as_deref(), Some("first person's note"));
    assert_eq!(b.rating, Some(Rating::Nope));
    assert_eq!(b.reading_status, Some(ReadingStatus::Read));
    assert_eq!(b.notes.as_deref(), Some("second person's note"));
    assert_eq!(
        books::list(&pool, BookFilter::default())
            .await
            .unwrap()
            .len(),
        1
    );
    // Status changes do not introduce event/date tables or implied history.
    let event_tables: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reading_event")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(event_tables, 0);
    pool.close().await;
}

#[tokio::test]
async fn duplicate_hints_never_merge_editions_and_isbn_is_optional() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let one = books::create(&pool, title("Same title"), None)
        .await
        .unwrap();
    let two = books::create(&pool, title("Same title"), None)
        .await
        .unwrap();
    assert_ne!(one, two);
    assert_eq!(
        books::duplicate_hints(&pool, title("  SAME   TITLE "), None)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        books::duplicate_hints(&pool, title("Same title"), Some(&one))
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        books::duplicate_hints(&pool, title("Same title extra"), None)
            .await
            .unwrap()
            .is_empty()
    );
    let draft = BookDraft {
        title: "ISBN edition".into(),
        isbn13: Some("978-0-306-40615-7".into()),
        ..Default::default()
    };
    let id = books::create(&pool, draft, None).await.unwrap();
    assert_eq!(
        books::get(&pool, &id)
            .await
            .unwrap()
            .book
            .data
            .isbn13
            .as_deref(),
        Some("9780306406157")
    );
    let hints = books::duplicate_hints(
        &pool,
        BookDraft {
            title: "Different title".into(),
            isbn13: Some("9780306406157".into()),
            ..Default::default()
        },
        None,
    )
    .await
    .unwrap();
    assert_eq!(hints[0].id, id);
    pool.close().await;
}

#[tokio::test]
async fn filters_use_one_membership_and_search_respects_person_context() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let first = person(&pool, "Reader A").await;
    let second = person(&pool, "Reader B").await;
    let book = books::create(&pool, title("Café book"), Some(&first))
        .await
        .unwrap();
    books::update_personal(
        &pool,
        &first,
        &book,
        PersonalDraft {
            reading_status: Some(ReadingStatus::Read),
            notes: Some("Alpha private-context phrase".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    books::update_personal(
        &pool,
        &second,
        &book,
        PersonalDraft {
            rating: Some(Rating::Loved),
            notes: Some("Beta context phrase".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(books::list(
        &pool,
        BookFilter {
            reading_status: Filter::Equals(ReadingStatus::Read),
            rating: Filter::Equals(Rating::Loved),
            ..Default::default()
        }
    )
    .await
    .unwrap()
    .is_empty());
    assert_eq!(
        books::list(
            &pool,
            BookFilter {
                person_id: Some(first.clone()),
                rating: Filter::Unknown,
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .len(),
        1
    );
    assert!(books::list(
        &pool,
        BookFilter {
            person_id: Some(second.clone()),
            search: "ALPHA".into(),
            ..Default::default()
        }
    )
    .await
    .unwrap()
    .is_empty());
    assert_eq!(
        books::list(
            &pool,
            BookFilter {
                person_id: Some(second),
                search: "CAFÉ".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .len(),
        1
    );
    assert!(books::list(
        &pool,
        BookFilter {
            owned: Filter::Equals(false),
            ..Default::default()
        }
    )
    .await
    .unwrap()
    .is_empty());
    assert_eq!(
        books::list(
            &pool,
            BookFilter {
                owned: Filter::Unknown,
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .len(),
        1
    );
    pool.close().await;
}

#[tokio::test]
async fn invalid_assignments_roll_back_book_creation_and_archives_reject_writes() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let missing = uuid::Uuid::new_v4().to_string();
    assert!(books::create(&pool, title("Rollback"), Some(&missing))
        .await
        .is_err());
    assert!(books::list(&pool, BookFilter::default())
        .await
        .unwrap()
        .is_empty());
    for draft in [
        title(" "),
        title("a\0b"),
        BookDraft {
            isbn13: Some("0000000000000".into()),
            ..title("Bad ISBN")
        },
        BookDraft {
            publication_year: Some(0),
            ..title("Bad year")
        },
    ] {
        assert_eq!(
            books::create(&pool, draft, None).await.unwrap_err().code,
            ErrorCode::Validation
        );
    }
    let person = person(&pool, "Reader").await;
    let book = books::create(&pool, title("Preserved"), Some(&person))
        .await
        .unwrap();
    household::archive(&pool, &person).await.unwrap();
    assert!(books::mark_read(&pool, &person, &book).await.is_err());
    assert!(
        books::update_personal(&pool, &person, &book, PersonalDraft::default())
            .await
            .is_err()
    );
    assert_eq!(
        books::get(&pool, &book).await.unwrap().memberships[0].reading_status,
        None
    );
    assert!(serde_json::from_str::<PersonalDraft>(r#"{"rating":"5"}"#).is_err());
    pool.close().await;
}

#[tokio::test]
async fn categorical_sort_and_nullable_filters_have_explicit_semantics() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let person = person(&pool, "Reader").await;
    for (name, rating, owned) in [
        ("Unknown", None, None),
        ("Nope", Some(Rating::Nope), Some(false)),
        ("Loved", Some(Rating::Loved), Some(true)),
    ] {
        let book = books::create(
            &pool,
            BookDraft {
                owned,
                ..title(name)
            },
            Some(&person),
        )
        .await
        .unwrap();
        books::update_personal(
            &pool,
            &person,
            &book,
            PersonalDraft {
                rating,
                list_status: Some(ListStatus::Wishlist),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    }
    let rows = books::list(
        &pool,
        BookFilter {
            person_id: Some(person.clone()),
            sort: BookSort::Rating,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        rows.iter()
            .map(|row| row.book.data.title.as_str())
            .collect::<Vec<_>>(),
        ["Loved", "Nope", "Unknown"]
    );
    let rows = books::list(
        &pool,
        BookFilter {
            person_id: Some(person.clone()),
            owned: Filter::Equals(false),
            list_status: Filter::Equals(ListStatus::Wishlist),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].book.data.title, "Nope");
    let rows = books::list(
        &pool,
        BookFilter {
            sort: BookSort::Title,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(rows[0].book.data.title, "Loved");
    assert!(books::list(
        &pool,
        BookFilter {
            list_status: Filter::Unknown,
            ..Default::default()
        }
    )
    .await
    .unwrap()
    .is_empty());
    assert!(books::list(
        &pool,
        BookFilter {
            search: "'; DROP TABLE book; --".into(),
            ..Default::default()
        }
    )
    .await
    .unwrap()
    .is_empty());
    assert_eq!(
        books::list(&pool, BookFilter::default())
            .await
            .unwrap()
            .len(),
        3
    );
    pool.close().await;
}

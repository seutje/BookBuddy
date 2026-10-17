use bookbuddy_lib::{
    domain::{
        books::{BookDraft, BookFilter, PersonalDraft, ReadingStatus},
        custom_fields::*,
    },
    persistence,
    services::{books, custom_fields as fields, household},
};
use serde_json::{json, Value};
use sqlx::SqlitePool;

fn draft(label: &str, scope: Scope, kind: Kind) -> FieldDraft {
    FieldDraft {
        scope,
        kind,
        label: label.into(),
        description: String::new(),
        settings: FieldSettings::default(),
        default_value: None,
        options: Vec::new(),
        order_index: 0,
        active: true,
        show_detail: true,
        show_in_rows: false,
        show_in_filters: true,
    }
}
async fn person(pool: &SqlitePool, name: &str) -> String {
    household::create(pool, name).await.unwrap();
    household::list(pool)
        .await
        .unwrap()
        .current_person_id
        .unwrap()
}
async fn book(pool: &SqlitePool, person: Option<&str>) -> String {
    books::create(
        pool,
        BookDraft {
            title: "Synthetic book".into(),
            ..Default::default()
        },
        person,
    )
    .await
    .unwrap()
}
fn target(person: &str, book: &str) -> ValueTarget {
    ValueTarget::PersonBook {
        person_id: person.into(),
        book_id: book.into(),
    }
}
async fn change(pool: &SqlitePool, id: &str, draft: FieldDraft, confirm: bool) {
    let preview = fields::preview_change(pool, id, draft.clone())
        .await
        .unwrap();
    fields::update(pool, id, draft, &preview.token, confirm)
        .await
        .unwrap();
}
fn filter(person: &str, field: &str, condition: CustomMatch) -> BookFilter {
    BookFilter {
        person_id: Some(person.into()),
        custom_filters: vec![CustomFilter {
            field_id: field.into(),
            condition,
        }],
        ..Default::default()
    }
}

#[tokio::test]
async fn a_new_field_is_sparse_on_hundreds_of_old_and_future_memberships() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let reader = person(&pool, "Reader").await;
    let mut books_ids = Vec::new();
    for _ in 0..300 {
        books_ids.push(book(&pool, Some(&reader)).await);
    }
    let schema_before: i64 = sqlx::query_scalar("PRAGMA schema_version")
        .fetch_one(&pool)
        .await
        .unwrap();
    let mut definition = draft("Recommend?", Scope::PersonBook, Kind::Boolean);
    definition.default_value = Some(json!(true));
    definition.show_in_rows = true;
    let id = fields::create(&pool, definition).await.unwrap();
    let schema_after: i64 = sqlx::query_scalar("PRAGMA schema_version")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(schema_before, schema_after);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM person_book_custom_field_value")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let values = fields::values(&pool, target(&reader, &books_ids[0]))
        .await
        .unwrap();
    assert_eq!(values[0].value, None);
    assert_eq!(values[0].definition.draft.default_value, Some(json!(true)));
    books_ids.push(book(&pool, Some(&reader)).await);
    fields::set_value(
        &pool,
        target(&reader, &books_ids[0]),
        &id,
        Some(json!(false)),
    )
    .await
    .unwrap();
    fields::set_value(
        &pool,
        target(&reader, &books_ids[1]),
        &id,
        Some(json!(true)),
    )
    .await
    .unwrap();
    assert_eq!(
        books::list(&pool, filter(&reader, &id, CustomMatch::Unknown))
            .await
            .unwrap()
            .len(),
        299
    );
    let no = books::list(
        &pool,
        filter(&reader, &id, CustomMatch::Equals(json!(false))),
    )
    .await
    .unwrap();
    assert_eq!(no.len(), 1);
    assert_eq!(no[0].row_fields[0].display, "No");
    assert_eq!(
        books::list(
            &pool,
            filter(&reader, &id, CustomMatch::Equals(json!(true)))
        )
        .await
        .unwrap()
        .len(),
        1
    );
    assert!(
        fields::set_value(&pool, target(&reader, &books_ids[0]), &id, Some(json!(0)))
            .await
            .is_err()
    );
    assert!(fields::set_value(
        &pool,
        target(&reader, &books_ids[0]),
        &id,
        Some(Value::Null)
    )
    .await
    .is_err());
    pool.close().await;
    let pool = persistence::open(directory.path()).await.unwrap();
    assert_eq!(
        fields::values(&pool, target(&reader, &books_ids[0]))
            .await
            .unwrap()[0]
            .value,
        Some(json!(false))
    );
    assert_eq!(
        fields::values(&pool, target(&reader, &books_ids[300]))
            .await
            .unwrap()[0]
            .value,
        None
    );
    pool.close().await;
}

#[tokio::test]
async fn scope_ids_and_archived_people_are_validated_in_rust_and_sqlite() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let reader = person(&pool, "Reader").await;
    let other = person(&pool, "Other").await;
    let book = book(&pool, Some(&reader)).await;
    let shared = fields::create(&pool, draft("Shared", Scope::Book, Kind::Boolean))
        .await
        .unwrap();
    let personal = fields::create(&pool, draft("Personal", Scope::PersonBook, Kind::Boolean))
        .await
        .unwrap();
    assert!(
        fields::set_value(&pool, target(&reader, &book), &shared, Some(json!(false)))
            .await
            .is_err()
    );
    assert!(fields::set_value(
        &pool,
        ValueTarget::Book {
            book_id: book.clone()
        },
        &personal,
        Some(json!(false))
    )
    .await
    .is_err());
    assert!(fields::set_value(
        &pool,
        target(&reader, &book),
        &uuid::Uuid::new_v4().to_string(),
        Some(json!(false))
    )
    .await
    .is_err());
    assert!(
        fields::set_value(&pool, target(&other, &book), &personal, Some(json!(false)))
            .await
            .is_err()
    );
    assert!(
        sqlx::query("INSERT INTO book_custom_field_value VALUES (?, ?, 'false', 'synthetic')")
            .bind(&book)
            .bind(&personal)
            .execute(&pool)
            .await
            .is_err()
    );
    let membership: String = sqlx::query_scalar("SELECT id FROM person_book WHERE person_id = ?")
        .bind(&reader)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(sqlx::query(
        "INSERT INTO person_book_custom_field_value VALUES (?, ?, 'false', 'synthetic')"
    )
    .bind(membership)
    .bind(&shared)
    .execute(&pool)
    .await
    .is_err());
    household::archive(&pool, &reader).await.unwrap();
    assert!(
        fields::set_value(&pool, target(&reader, &book), &personal, Some(json!(true)))
            .await
            .is_err()
    );
    pool.close().await;
}

#[tokio::test]
async fn select_rename_retirement_hide_and_reactivation_preserve_identity_and_values() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let book = book(&pool, None).await;
    let target = ValueTarget::Book {
        book_id: book.clone(),
    };
    let option = uuid::Uuid::new_v4().to_string();
    let mut definition = draft("Format", Scope::Book, Kind::Select);
    definition.options.push(FieldOption {
        id: option.clone(),
        label: "Board".into(),
        active: true,
    });
    let id = fields::create(&pool, definition.clone()).await.unwrap();
    fields::set_value(&pool, target.clone(), &id, Some(json!(option)))
        .await
        .unwrap();
    definition.label = "Book format".into();
    definition.options[0].label = "Board book".into();
    change(&pool, &id, definition.clone(), false).await;
    let values = fields::values(&pool, target.clone()).await.unwrap();
    assert_eq!(values[0].value, Some(json!(option)));
    assert_eq!(values[0].definition.draft.options[0].label, "Board book");
    definition.options[0].active = false;
    let preview = fields::preview_change(&pool, &id, definition.clone())
        .await
        .unwrap();
    assert!(preview.requires_confirmation);
    assert_eq!(preview.cleared_value_count, 0);
    assert!(
        fields::update(&pool, &id, definition.clone(), &preview.token, false)
            .await
            .is_err()
    );
    fields::update(&pool, &id, definition.clone(), &preview.token, true)
        .await
        .unwrap();
    assert_eq!(
        fields::values(&pool, target.clone()).await.unwrap()[0].value,
        Some(json!(option))
    );
    assert!(
        fields::set_value(&pool, target.clone(), &id, Some(json!(option)))
            .await
            .is_err()
    );
    let another = uuid::Uuid::new_v4().to_string();
    assert!(
        fields::set_value(&pool, target.clone(), &id, Some(json!(another)))
            .await
            .is_err()
    );
    definition.active = false;
    change(&pool, &id, definition.clone(), true).await;
    assert!(fields::values(&pool, target.clone())
        .await
        .unwrap()
        .is_empty());
    assert!(fields::set_value(&pool, target.clone(), &id, None)
        .await
        .is_err());
    definition.active = true;
    change(&pool, &id, definition, false).await;
    pool.close().await;
    let pool = persistence::open(directory.path()).await.unwrap();
    assert_eq!(
        fields::values(&pool, target).await.unwrap()[0].value,
        Some(json!(option))
    );
    pool.close().await;
}

#[tokio::test]
async fn destructive_changes_require_fresh_preview_and_failed_updates_roll_back_clears() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let book = book(&pool, None).await;
    let target = ValueTarget::Book { book_id: book };
    let definition = draft("Flag", Scope::Book, Kind::Boolean);
    let id = fields::create(&pool, definition.clone()).await.unwrap();
    fields::set_value(&pool, target.clone(), &id, Some(json!(false)))
        .await
        .unwrap();
    let mut renamed = definition.clone();
    renamed.label = "Renamed".into();
    let stale = fields::preview_change(&pool, &id, renamed.clone())
        .await
        .unwrap();
    fields::set_value(&pool, target.clone(), &id, Some(json!(true)))
        .await
        .unwrap();
    assert!(fields::update(&pool, &id, renamed, &stale.token, true)
        .await
        .is_err());
    let mut cross_scope = definition.clone();
    cross_scope.scope = Scope::PersonBook;
    assert!(fields::preview_change(&pool, &id, cross_scope)
        .await
        .is_err());
    let mut new_type = draft("Flag", Scope::Book, Kind::Text);
    let impact = fields::preview_change(&pool, &id, new_type.clone())
        .await
        .unwrap();
    assert_eq!(impact.cleared_value_count, 1);
    assert!(
        fields::update(&pool, &id, new_type.clone(), &impact.token, false)
            .await
            .is_err()
    );
    fields::create(&pool, draft("Collision", Scope::Book, Kind::Text))
        .await
        .unwrap();
    new_type.label = "Collision".into();
    let impact = fields::preview_change(&pool, &id, new_type.clone())
        .await
        .unwrap();
    assert!(
        fields::update(&pool, &id, new_type.clone(), &impact.token, true)
            .await
            .is_err()
    );
    assert_eq!(
        fields::values(&pool, target.clone())
            .await
            .unwrap()
            .iter()
            .find(|value| value.definition.id == id)
            .unwrap()
            .value,
        Some(json!(true))
    );
    new_type.label = "Converted with review".into();
    change(&pool, &id, new_type, true).await;
    assert_eq!(
        fields::values(&pool, target)
            .await
            .unwrap()
            .iter()
            .find(|value| value.definition.id == id)
            .unwrap()
            .value,
        None
    );
    pool.close().await;
}

#[tokio::test]
async fn empty_zero_unknown_text_search_and_combined_filters_are_distinct() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let first = person(&pool, "First").await;
    let second = person(&pool, "Second").await;
    let book_id = book(&pool, Some(&first)).await;
    books::ensure_membership(&pool, &second, &book_id)
        .await
        .unwrap();
    let text = fields::create(&pool, draft("Text", Scope::PersonBook, Kind::Text))
        .await
        .unwrap();
    let number = fields::create(&pool, draft("Number", Scope::Book, Kind::Number))
        .await
        .unwrap();
    fields::set_value(&pool, target(&first, &book_id), &text, Some(json!("")))
        .await
        .unwrap();
    assert_eq!(
        books::list(&pool, filter(&first, &text, CustomMatch::Equals(json!(""))))
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        books::list(&pool, filter(&first, &text, CustomMatch::Unknown))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        books::list(&pool, filter(&second, &text, CustomMatch::Unknown))
            .await
            .unwrap()
            .len(),
        1
    );
    fields::set_value(
        &pool,
        ValueTarget::Book {
            book_id: book_id.clone(),
        },
        &number,
        Some(json!(0)),
    )
    .await
    .unwrap();
    let mut query = BookFilter {
        custom_filters: vec![CustomFilter {
            field_id: number,
            condition: CustomMatch::Equals(json!(0.0)),
        }],
        ..Default::default()
    };
    assert_eq!(books::list(&pool, query.clone()).await.unwrap().len(), 1);
    fields::set_value(
        &pool,
        target(&first, &book_id),
        &text,
        Some(json!("CAFÉ notes")),
    )
    .await
    .unwrap();
    query = BookFilter {
        person_id: Some(second.clone()),
        search: "café".into(),
        ..Default::default()
    };
    assert!(books::list(&pool, query).await.unwrap().is_empty());
    assert_eq!(
        books::list(
            &pool,
            BookFilter {
                search: "café".into(),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .len(),
        1
    );
    books::update_personal(
        &pool,
        &second,
        &book_id,
        PersonalDraft {
            reading_status: Some(ReadingStatus::Read),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let mut query = filter(&first, &text, CustomMatch::Equals(json!("CAFÉ notes")));
    query.person_id = None;
    query.reading_status = bookbuddy_lib::domain::books::Filter::Equals(ReadingStatus::Read);
    assert!(books::list(&pool, query).await.unwrap().is_empty());
    fields::set_value(&pool, target(&first, &book_id), &text, None)
        .await
        .unwrap();
    assert_eq!(
        fields::values(&pool, target(&first, &book_id))
            .await
            .unwrap()[0]
            .value,
        None
    );
    pool.close().await;
}

#[test]
fn every_supported_type_enforces_its_shape_bounds_and_calendar() {
    let boolean = draft("Boolean", Scope::Book, Kind::Boolean);
    assert!(boolean.validate_value(&json!(false), false).is_ok());
    assert!(boolean.validate_value(&json!(0), false).is_err());
    let mut number = draft("Number", Scope::Book, Kind::Number);
    number.settings.min = Some(0.0);
    number.settings.max = Some(10.0);
    for value in [json!(0), json!(1.5), json!(10)] {
        assert!(number.validate_value(&value, false).is_ok());
    }
    for value in [json!(-1), json!(11), json!("2"), Value::Null] {
        assert!(number.validate_value(&value, false).is_err());
    }
    let date = draft("Date", Scope::Book, Kind::Date);
    assert!(date.validate_value(&json!("2024-02-29"), false).is_ok());
    for value in [
        "2023-02-29",
        "2024-2-29",
        "0000-01-01",
        "2024-01-01T00:00:00Z",
    ] {
        assert!(date.validate_value(&json!(value), false).is_err());
    }
    let mut rating = draft("Rating", Scope::Book, Kind::Rating);
    assert!(rating.validate_value(&json!(5), false).is_ok());
    assert!(rating.validate_value(&json!(5.5), false).is_err());
    rating.settings.min = Some(0.0);
    rating.settings.max = Some(10.0);
    assert!(rating.validate().is_ok());
    let mut text = draft("Text", Scope::Book, Kind::Text);
    assert!(text.validate_value(&json!(""), false).is_ok());
    assert!(text.validate_value(&json!("two\nlines"), false).is_err());
    text.settings.multiline = true;
    assert!(text.validate_value(&json!("two\nlines"), false).is_ok());
}

#[tokio::test]
async fn label_collisions_option_ownership_and_reordering_are_safe() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let first = fields::create(&pool, draft("Recommend?", Scope::Book, Kind::Boolean))
        .await
        .unwrap();
    assert!(
        fields::create(&pool, draft(" RECOMMEND? ", Scope::Book, Kind::Boolean))
            .await
            .is_err()
    );
    let second = fields::create(&pool, draft("Recommend?", Scope::PersonBook, Kind::Boolean))
        .await
        .unwrap();
    assert!(fields::reorder(&pool, vec![first.clone(), first.clone()])
        .await
        .is_err());
    fields::reorder(&pool, vec![second.clone(), first.clone()])
        .await
        .unwrap();
    assert_eq!(fields::list(&pool).await.unwrap()[0].id, second);
    let mut select = draft("Choice", Scope::Book, Kind::Select);
    select.options.push(FieldOption {
        id: uuid::Uuid::new_v4().to_string(),
        label: "One".into(),
        active: true,
    });
    fields::create(&pool, select.clone()).await.unwrap();
    select.label = "Another field".into();
    assert!(fields::create(&pool, select).await.is_err());
    pool.close().await;
}

#[tokio::test]
async fn unit_label_changes_preserve_values_while_tighter_bounds_require_review() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let book = book(&pool, None).await;
    let target = ValueTarget::Book { book_id: book };
    let mut definition = draft("Length", Scope::Book, Kind::Number);
    definition.settings.unit = Some("pages".into());
    let id = fields::create(&pool, definition.clone()).await.unwrap();
    fields::set_value(&pool, target.clone(), &id, Some(json!(1.5)))
        .await
        .unwrap();
    definition.settings.unit = Some("units".into());
    let preview = fields::preview_change(&pool, &id, definition.clone())
        .await
        .unwrap();
    assert_eq!(preview.cleared_value_count, 0);
    assert!(!preview.requires_confirmation);
    fields::update(&pool, &id, definition.clone(), &preview.token, false)
        .await
        .unwrap();
    assert_eq!(
        fields::values(&pool, target.clone()).await.unwrap()[0].value,
        Some(json!(1.5))
    );
    definition.settings.max = Some(1.0);
    let preview = fields::preview_change(&pool, &id, definition.clone())
        .await
        .unwrap();
    assert_eq!(preview.cleared_value_count, 1);
    assert!(preview.requires_confirmation);
    assert!(
        fields::update(&pool, &id, definition, &preview.token, false)
            .await
            .is_err()
    );
    assert_eq!(
        fields::values(&pool, target).await.unwrap()[0].value,
        Some(json!(1.5))
    );
    pool.close().await;
}

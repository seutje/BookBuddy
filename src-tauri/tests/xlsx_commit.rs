mod support;
use bookbuddy_lib::{
    domain::{
        books::{BookDraft, BookFilter, PersonalDraft, Rating, ReadingStatus},
        custom_fields::ValueTarget,
        import::*,
    },
    persistence,
    services::{
        books, custom_fields as fields, household, import_commit, import_history, import_preview,
        readings, xlsx_reader,
    },
};
use serde_json::json;
use sqlx::SqlitePool;
use support::xlsx::{self, text};

async fn ready(
    pool: &SqlitePool,
    workbook: &WorkbookData,
    draft: Option<ImportReviewDraft>,
) -> ImportPreview {
    let mut draft = match draft {
        Some(draft) => draft,
        None => import_preview::initial_draft(pool, &workbook.source_name, "test-selection")
            .await
            .unwrap(),
    };
    draft.acknowledge_warnings = true;
    draft.acknowledge_missing_sheets = true;
    let preview = import_preview::preview(pool, workbook, draft)
        .await
        .unwrap();
    assert!(preview.blocking_errors.is_empty());
    assert_eq!(preview.requires_title_decisions, 0);
    preview
}
async fn count(pool: &SqlitePool, table: &str) -> i64 {
    // Static test-only table choices; no command accepts SQL or a table name.
    let query = match table {
        "book" => "SELECT COUNT(*) FROM book",
        "person" => "SELECT COUNT(*) FROM person",
        "membership" => "SELECT COUNT(*) FROM person_book",
        "loan" => "SELECT COUNT(*) FROM library_borrow_event",
        "reading" => "SELECT COUNT(*) FROM reading_event",
        "batch" => "SELECT COUNT(*) FROM import_batch",
        "row" => "SELECT COUNT(*) FROM import_source_row",
        "field" => "SELECT COUNT(*) FROM custom_field_definition",
        "task" => "SELECT COUNT(*) FROM book_title_verification",
        _ => panic!("Unknown synthetic test table"),
    };
    sqlx::query_scalar(query).fetch_one(pool).await.unwrap()
}

#[tokio::test]
async fn commit_is_atomic_traceable_repeatable_and_does_not_invent_reading_events() {
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("db");
    let file = directory.path().join("synthetic.xlsx");
    xlsx::write(&file, &xlsx::family(), false);
    let pool = persistence::open(&db).await.unwrap();
    let workbook = xlsx_reader::inspect(&file).unwrap();
    let preview = ready(&pool, &workbook, None).await;
    let summary = import_commit::commit(
        &pool,
        &db,
        &file,
        &workbook,
        preview.draft.clone(),
        &preview.review_hash,
    )
    .await
    .unwrap();
    assert_eq!(summary.books_added, 2);
    assert_eq!(summary.people_created, 2);
    assert_eq!(summary.memberships_created, 2);
    assert_eq!(summary.historical_loans_added, 2);
    assert_eq!(summary.title_tasks_added, 1);
    assert_eq!(count(&pool, "reading").await, 0);
    assert_eq!(count(&pool, "loan").await, 2);
    assert_eq!(count(&pool, "row").await, 5);
    assert_eq!(count(&pool, "task").await, 1);
    assert_eq!(
        household::list(&pool).await.unwrap().current_person_id,
        Some(summary.carly_person_id.clone())
    );
    let mom_memberships: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM person_book pb JOIN person p ON p.id = pb.person_id WHERE p.display_name = 'Mom'").fetch_one(&pool).await.unwrap();
    assert_eq!(mom_memberships, 0);
    let original = books::list(&pool, BookFilter::default()).await.unwrap();
    let flagged = original
        .iter()
        .find(|row| row.book.data.title == "Synthetic board book")
        .unwrap();
    assert_eq!(flagged.book.read_count, 0);
    assert_eq!(flagged.book.last_read_on, None);
    assert!(flagged.book.needs_title_verification);
    let repeated = import_commit::commit(
        &pool,
        &db,
        &file,
        &workbook,
        preview.draft,
        &preview.review_hash,
    )
    .await
    .unwrap();
    assert!(repeated.already_imported);
    assert_eq!(repeated.books_added, 0);
    assert_eq!(count(&pool, "batch").await, 1);
    assert_eq!(count(&pool, "loan").await, 2);
    assert_eq!(count(&pool, "row").await, 5);
    let backup = db.join("backups").join(summary.backup_name.unwrap());
    let snapshot = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(backup)
                .read_only(true),
        )
        .await
        .unwrap();
    assert_eq!(count(&snapshot, "book").await, 0);
    assert_eq!(
        sqlx::query_scalar::<_, String>("PRAGMA integrity_check")
            .fetch_one(&snapshot)
            .await
            .unwrap(),
        "ok"
    );
    snapshot.close().await;
    assert!(sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&pool)
        .await
        .unwrap()
        .is_empty());
    pool.close().await;
}

#[tokio::test]
async fn changed_files_and_stale_library_reviews_leave_existing_data_safe() {
    use std::io::Write;
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("db");
    let file = directory.path().join("synthetic.xlsx");
    xlsx::write(&file, &xlsx::family(), false);
    let pool = persistence::open(&db).await.unwrap();
    let workbook = xlsx_reader::inspect(&file).unwrap();
    let preview = ready(&pool, &workbook, None).await;
    std::fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .unwrap()
        .write_all(b"changed")
        .unwrap();
    assert!(import_commit::commit(
        &pool,
        &db,
        &file,
        &workbook,
        preview.draft.clone(),
        &preview.review_hash
    )
    .await
    .is_err());
    assert_eq!(count(&pool, "book").await, 0);
    assert!(!db.join("backups").exists());
    xlsx::write(&file, &xlsx::family(), false);
    household::create(&pool, "Manual reader").await.unwrap();
    assert!(import_commit::commit(
        &pool,
        &db,
        &file,
        &workbook,
        preview.draft,
        &preview.review_hash
    )
    .await
    .is_err());
    assert_eq!(count(&pool, "person").await, 1);
    assert_eq!(count(&pool, "book").await, 0);
    assert!(!db.join("backups").exists());
    pool.close().await;
}

#[tokio::test]
async fn an_interrupted_transaction_rolls_back_all_import_entities_and_keeps_a_wal_safe_backup() {
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("db");
    let file = directory.path().join("synthetic.xlsx");
    xlsx::write(&file, &xlsx::family(), false);
    let pool = persistence::open(&db).await.unwrap();
    household::create(&pool, "Manual reader").await.unwrap();
    let manual = books::create(
        &pool,
        BookDraft {
            title: "Manual book".into(),
            notes: Some("Committed in WAL before backup".into()),
            ..Default::default()
        },
        None,
    )
    .await
    .unwrap();
    sqlx::query("CREATE TRIGGER synthetic_interruption BEFORE INSERT ON book WHEN NEW.title = 'Synthetic second book' BEGIN SELECT RAISE(ABORT, 'synthetic failure'); END").execute(&pool).await.unwrap();
    let workbook = xlsx_reader::inspect(&file).unwrap();
    let preview = ready(&pool, &workbook, None).await;
    assert!(import_commit::commit(
        &pool,
        &db,
        &file,
        &workbook,
        preview.draft,
        &preview.review_hash
    )
    .await
    .is_err());
    assert_eq!(count(&pool, "book").await, 1);
    assert_eq!(count(&pool, "person").await, 1);
    assert_eq!(count(&pool, "batch").await, 0);
    assert_eq!(count(&pool, "row").await, 0);
    assert_eq!(count(&pool, "loan").await, 0);
    assert_eq!(count(&pool, "field").await, 0);
    assert_eq!(
        books::get(&pool, &manual)
            .await
            .unwrap()
            .book
            .data
            .notes
            .as_deref(),
        Some("Committed in WAL before backup")
    );
    let backup = std::fs::read_dir(db.join("backups"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let backup_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(backup)
                .read_only(true),
        )
        .await
        .unwrap();
    assert_eq!(count(&backup_pool, "book").await, 1);
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT notes FROM book")
            .fetch_one(&backup_pool)
            .await
            .unwrap(),
        "Committed in WAL before backup"
    );
    assert_eq!(count(&backup_pool, "batch").await, 0);
    backup_pool.close().await;
    pool.close().await;
}

#[tokio::test]
async fn revised_sources_preserve_manual_values_stable_fields_options_and_legitimate_loan_repeats()
{
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("db");
    let file = directory.path().join("synthetic.xlsx");
    let mut sheets = xlsx::family();
    xlsx::write(&file, &sheets, false);
    let pool = persistence::open(&db).await.unwrap();
    let workbook = xlsx_reader::inspect(&file).unwrap();
    let preview = ready(&pool, &workbook, None).await;
    let first = import_commit::commit(
        &pool,
        &db,
        &file,
        &workbook,
        preview.draft,
        &preview.review_hash,
    )
    .await
    .unwrap();
    let original = books::list(&pool, BookFilter::default()).await.unwrap();
    let book_id = original
        .iter()
        .find(|row| row.book.data.title == "Synthetic board book")
        .unwrap()
        .book
        .id
        .clone();
    let mut manual = books::get(&pool, &book_id).await.unwrap().book.data;
    manual.title = "Manual shared title".into();
    manual.notes = Some("Manual shared note".into());
    manual.owned = Some(false);
    books::update(&pool, &book_id, manual).await.unwrap();
    readings::log_today(&pool, book_id.clone(), vec![first.carly_person_id.clone()])
        .await
        .unwrap();
    books::update_personal(
        &pool,
        &first.carly_person_id,
        &book_id,
        PersonalDraft {
            reading_status: Some(ReadingStatus::WantToRead),
            rating: Some(Rating::Nope),
            notes: Some("Manual personal note".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let mappings = persistence::import::field_mappings(&pool, &first.origin_id)
        .await
        .unwrap();
    let favourite_id = &mappings["favourite"];
    let format_id = &mappings["format"];
    fields::set_value(
        &pool,
        ValueTarget::PersonBook {
            person_id: first.carly_person_id.clone(),
            book_id: book_id.clone(),
        },
        favourite_id,
        Some(json!(false)),
    )
    .await
    .unwrap();
    let definition = fields::list(&pool)
        .await
        .unwrap()
        .into_iter()
        .find(|field| &field.id == format_id)
        .unwrap();
    let option_id = definition
        .draft
        .options
        .iter()
        .find(|option| option.label == "Board Book")
        .unwrap()
        .id
        .clone();
    let mut renamed = definition.draft;
    renamed.label = "Physical format".into();
    renamed
        .options
        .iter_mut()
        .find(|option| option.id == option_id)
        .unwrap()
        .label = "Board edition".into();
    let impact = fields::preview_change(&pool, format_id, renamed.clone())
        .await
        .unwrap();
    fields::update(&pool, format_id, renamed, &impact.token, false)
        .await
        .unwrap();
    sheets[0].1[1][17] = text("Revised source note");
    sheets[0].1[1][6] = text("Loved");
    sheets[0].1[1][7] = text("Liked");
    let mut added = sheets[0].1[1].clone();
    added[0] = text("New source edition");
    added[18] = text("Distinct new source label");
    added[19] = text("No");
    sheets[0].1.push(added);
    let loan = sheets[1].1[1].clone();
    sheets[1].1.push(loan);
    sheets[2].1[1][4] = text("Revised confirmed source title");
    xlsx::write(&file, &sheets, false);
    let workbook = xlsx_reader::inspect(&file).unwrap();
    let preview = ready(&pool, &workbook, None).await;
    assert_eq!(
        preview.draft.source_origin_id.as_deref(),
        Some(first.origin_id.as_str())
    );
    let second = import_commit::commit(
        &pool,
        &db,
        &file,
        &workbook,
        preview.draft,
        &preview.review_hash,
    )
    .await
    .unwrap();
    assert_eq!(second.people_created, 0);
    assert_eq!(second.books_added, 1);
    assert_eq!(second.historical_loans_added, 1);
    assert_eq!(second.historical_loans_reused, 2);
    let result = books::get(&pool, &book_id).await.unwrap();
    assert_eq!(result.book.data.title, "Manual shared title");
    assert_eq!(
        result.book.data.notes.as_deref(),
        Some("Manual shared note")
    );
    assert_eq!(result.book.data.owned, Some(false));
    assert_eq!(result.book.read_count, 1);
    let carly = result
        .memberships
        .iter()
        .find(|membership| membership.person_id == first.carly_person_id)
        .unwrap();
    assert_eq!(carly.rating, Some(Rating::Nope));
    assert_eq!(carly.reading_status, Some(ReadingStatus::WantToRead));
    assert_eq!(carly.notes.as_deref(), Some("Manual personal note"));
    let mom = result
        .memberships
        .iter()
        .find(|membership| membership.person_id != first.carly_person_id)
        .unwrap();
    assert_eq!(mom.rating, Some(Rating::Liked));
    assert_eq!(mom.reading_status, None);
    let values = fields::values(
        &pool,
        ValueTarget::PersonBook {
            person_id: first.carly_person_id.clone(),
            book_id: book_id.clone(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        values
            .iter()
            .find(|value| &value.definition.id == favourite_id)
            .unwrap()
            .value,
        Some(json!(false))
    );
    let new_book = books::list(&pool, BookFilter::default())
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.book.data.title == "New source edition")
        .unwrap();
    let values = fields::values(
        &pool,
        ValueTarget::Book {
            book_id: new_book.book.id,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        values
            .iter()
            .find(|value| &value.definition.id == format_id)
            .unwrap()
            .value,
        Some(json!(option_id))
    );
    assert_eq!(count(&pool, "task").await, 1);
    assert_eq!(count(&pool, "reading").await, 1);
    sheets[3].1.push(vec![text("Revised reference only")]);
    xlsx::write(&file, &sheets, false);
    let workbook = xlsx_reader::inspect(&file).unwrap();
    let mut draft = import_preview::initial_draft(&pool, &workbook.source_name, "test")
        .await
        .unwrap();
    let preview = ready(&pool, &workbook, Some(draft.clone())).await;
    let title = &preview.title_reviews[0];
    draft.decisions.push(RowDecision {
        row_key: title.source.key.clone(),
        target: title.target.clone(),
        accept_confirmed_title: true,
    });
    let preview = ready(&pool, &workbook, Some(draft)).await;
    import_commit::commit(
        &pool,
        &db,
        &file,
        &workbook,
        preview.draft,
        &preview.review_hash,
    )
    .await
    .unwrap();
    assert_eq!(
        books::get(&pool, &book_id).await.unwrap().book.data.title,
        "Revised confirmed source title"
    );
    assert_eq!(count(&pool, "task").await, 1);
    assert_eq!(count(&pool, "loan").await, 3);
    assert_eq!(count(&pool, "reading").await, 1);
    pool.close().await;
}

#[tokio::test]
async fn a_file_change_during_transaction_preparation_rolls_back_before_final_commit() {
    use std::io::Write;
    let directory = tempfile::tempdir().unwrap();
    let db = directory.path().join("db");
    let file = directory.path().join("synthetic.xlsx");
    let mut sheets = xlsx::family();
    for index in 0..100 {
        let mut row = sheets[0].1[1].clone();
        row[0] = text(&format!("Generated source {index}"));
        row[18] = text(&format!("Distinct source {index}"));
        row[19] = text("No");
        sheets[0].1.push(row);
    }
    xlsx::write(&file, &sheets, false);
    let pool = persistence::open(&db).await.unwrap();
    let workbook = xlsx_reader::inspect(&file).unwrap();
    let preview = ready(&pool, &workbook, None).await;
    let watched = db.join("backups");
    let changed_file = file.clone();
    let changer = tokio::spawn(async move {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if std::fs::read_dir(&watched)
                .ok()
                .and_then(|mut entries| entries.next())
                .is_some()
            {
                std::fs::OpenOptions::new()
                    .append(true)
                    .open(changed_file)
                    .unwrap()
                    .write_all(b"changed during preparation")
                    .unwrap();
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    });
    let result = import_commit::commit(
        &pool,
        &db,
        &file,
        &workbook,
        preview.draft,
        &preview.review_hash,
    )
    .await;
    changer.await.unwrap();
    assert!(result.is_err());
    assert_eq!(count(&pool, "book").await, 0);
    assert_eq!(count(&pool, "batch").await, 0);
    assert_eq!(count(&pool, "person").await, 0);
    pool.close().await;
}

#[tokio::test]
#[ignore = "Private source is committed only to a disposable test database; no source rows are printed"]
async fn private_sample_commit_and_exact_repeat_match_all_documented_counts() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let file = root
        .join("example")
        .join("Carly's Little Library import.xlsx");
    let workbook = xlsx_reader::inspect(&file).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let preview = ready(&pool, &workbook, None).await;
    let summary = import_commit::commit(
        &pool,
        directory.path(),
        &file,
        &workbook,
        preview.draft.clone(),
        &preview.review_hash,
    )
    .await
    .unwrap();
    assert_eq!(summary.books_added, 312);
    assert_eq!(count(&pool, "loan").await, 253);
    assert_eq!(count(&pool, "row").await, 600);
    assert_eq!(count(&pool, "reading").await, 0);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM import_source_row WHERE sheet_name = 'Check Titles'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        35
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM book WHERE owned = 1")
            .fetch_one(&pool)
            .await
            .unwrap(),
        66
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM person_book WHERE reading_status = 'currently_borrowed'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        3
    );
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM person_book_custom_field_value v JOIN import_field_mapping m ON m.field_id = v.field_id WHERE m.source_key = 'favourite' AND v.value_json = 'true'").fetch_one(&pool).await.unwrap(), 37);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM person_book WHERE rating IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let repeat = import_commit::commit(
        &pool,
        directory.path(),
        &file,
        &workbook,
        preview.draft,
        &preview.review_hash,
    )
    .await
    .unwrap();
    assert!(repeat.already_imported);
    assert_eq!(count(&pool, "book").await, 312);
    assert_eq!(count(&pool, "loan").await, 253);
    assert_eq!(count(&pool, "membership").await, 312);
    assert_eq!(count(&pool, "batch").await, 1);
    assert!(sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(&pool)
        .await
        .unwrap()
        .is_empty());
    pool.close().await;
}

#[tokio::test]
async fn title_and_loan_review_preserve_source_and_reject_stale_decisions() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("synthetic.xlsx");
    xlsx::write(&file, &xlsx::family(), false);
    let pool = persistence::open(&directory.path().join("db"))
        .await
        .unwrap();
    let workbook = xlsx_reader::inspect(&file).unwrap();
    let preview = ready(&pool, &workbook, None).await;
    import_commit::commit(
        &pool,
        &directory.path().join("db"),
        &file,
        &workbook,
        preview.draft,
        &preview.review_hash,
    )
    .await
    .unwrap();
    let task = import_history::tasks(&pool, false).await.unwrap().remove(0);
    let book = books::list(&pool, BookFilter::default())
        .await
        .unwrap()
        .into_iter()
        .find(|row| Some(&row.book.id) == task.book_id.as_ref())
        .unwrap()
        .book;
    let resolution = TitleResolution {
        book_id: task.book_id.clone(),
        confirmed_title: Some("Reviewed full title".into()),
        expected_current_title: Some(book.data.title.clone()),
        expected_state: "open".into(),
    };
    let mut stale = resolution.clone();
    stale.expected_current_title = Some("Stale title".into());
    assert!(import_history::resolve_title(&pool, &task.id, stale)
        .await
        .is_err());
    import_history::resolve_title(&pool, &task.id, resolution.clone())
        .await
        .unwrap();
    assert!(import_history::resolve_title(&pool, &task.id, resolution)
        .await
        .is_err());
    assert!(import_history::tasks(&pool, false)
        .await
        .unwrap()
        .is_empty());
    let resolved = import_history::tasks(&pool, true).await.unwrap().remove(0);
    assert_eq!(resolved.working_title, task.working_title);
    assert_eq!(resolved.raw_pdf_title, task.raw_pdf_title);
    assert_eq!(
        resolved.current_title.as_deref(),
        Some("Reviewed full title")
    );
    let filtered = books::list(
        &pool,
        BookFilter {
            needs_title_verification: Some(true),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(filtered.is_empty());
    let other = books::create(
        &pool,
        BookDraft {
            title: "Separate edition".into(),
            ..Default::default()
        },
        None,
    )
    .await
    .unwrap();
    let loan = import_history::loans(&pool, None).await.unwrap().remove(0);
    import_history::link_loan(&pool, &loan.id, &other, loan.book_id.as_deref())
        .await
        .unwrap();
    assert!(
        import_history::link_loan(&pool, &loan.id, &book.id, loan.book_id.as_deref())
            .await
            .is_err()
    );
    let updated = import_history::loans(&pool, Some(&other))
        .await
        .unwrap()
        .remove(0);
    assert_eq!(updated.source_title, loan.source_title);
    assert_eq!(updated.source_date_json, loan.source_date_json);
    assert_eq!(count(&pool, "reading").await, 0);
    pool.close().await;
}

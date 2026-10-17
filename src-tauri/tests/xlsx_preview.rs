mod support;
use bookbuddy_lib::{
    domain::{books::Rating, import::*},
    persistence,
    services::{household, import_preview, xlsx_reader},
};
use support::xlsx::{self, text, Cell};

async fn run(sheets: &[(String, Vec<Vec<Cell>>)], date_1904: bool) -> ImportPreview {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("synthetic.xlsx");
    xlsx::write(&file, sheets, date_1904);
    let workbook = xlsx_reader::inspect(&file).unwrap();
    let database = directory.path().join("database");
    let pool = persistence::open(&database).await.unwrap();
    pool.close().await;
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(database.join("bookbuddy.sqlite3"))
        .read_only(true)
        .foreign_keys(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .unwrap();
    let preview = import_preview::preview(
        &pool,
        &workbook,
        ImportReviewDraft::initial(&uuid::Uuid::new_v4().to_string()),
    )
    .await
    .unwrap();
    assert!(household::list(&pool).await.unwrap().people.is_empty());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM book")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM custom_field_definition")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    pool.close().await;
    preview
}

#[tokio::test]
async fn all_five_sheets_and_every_catalogue_mapping_are_previewed_without_writes() {
    let preview = run(&xlsx::family(), false).await;
    assert!(preview.blocking_errors.is_empty());
    assert_eq!(preview.sheets.len(), 5);
    assert_eq!(preview.counts.catalogue, 2);
    assert_eq!(preview.counts.historical_loans, 2);
    assert_eq!(preview.counts.title_reviews, 1);
    let first = &preview.catalogue[0];
    assert_eq!(first.book.authors.as_deref(), Some("Example, Author"));
    assert_eq!(first.book.publication_year, Some(2020));
    assert_eq!(first.book.owned, Some(true));
    assert_eq!(first.carly_rating, None);
    assert_eq!(first.mom_rating, None);
    assert_eq!(first.favourite, Some(true));
    assert_eq!(first.renewed_to_death, Some(false));
    assert_eq!(first.borrow_count, Some(3));
    assert_eq!(first.reading_effort.as_deref(), Some("Short"));
    assert_eq!(first.age_range.as_deref(), Some("Toddler"));
    assert_eq!(first.buy_status.as_deref(), Some("Purchased"));
    assert_eq!(first.book.notes.as_deref(), Some("Shared source note"));
    assert_eq!(first.original_title.as_deref(), Some("Clipped PDF label"));
    assert_eq!(first.check_title, Some(true));
    assert_eq!(preview.catalogue[1].book.authors, None);
    assert_eq!(preview.catalogue[1].book.owned, Some(false));
    assert_eq!(preview.counts.mom_memberships, 0);
    assert_eq!(
        preview.history[0].source_date.value.as_deref(),
        Some("2023-03-15")
    );
    assert_eq!(
        preview.history[0].source_date.raw.as_ref().unwrap().kind,
        "excel_date"
    );
    assert_eq!(
        preview.history[0].date_type.as_deref(),
        Some("Historical return date")
    );
    assert_eq!(
        preview.history[0].source_pdf.as_deref(),
        Some("synthetic-source.pdf")
    );
    assert_ne!(preview.history[0].source.key, preview.history[1].source.key);
    assert_eq!(
        preview.history[0].source.fingerprint,
        preview.history[1].source.fingerprint
    );
    assert_eq!(preview.history[1].source.occurrence, 2);
    assert!(preview
        .warnings
        .iter()
        .any(|warning| warning.code == "borrow_count_mismatch"));
    assert_eq!(
        preview.title_reviews[0].confirmed_title.as_deref(),
        Some("Synthetic confirmed title")
    );
    assert!(!preview.title_reviews[0].accept_confirmed_title);
    assert_eq!(
        preview.title_reviews[0].raw_pdf_title.as_deref(),
        Some("Clipped PDF label")
    );
}

#[tokio::test]
async fn reordered_shifted_headers_unknown_columns_and_blank_rows_preserve_source_values() {
    let mut sheets = xlsx::family();
    for row in &mut sheets[0].1 {
        row.reverse();
        row.insert(0, Cell::Empty);
    }
    sheets[0].1[0].push(text("Extra source column"));
    sheets[0].1[1].push(text("Must remain in provenance"));
    sheets[0].1.insert(0, vec![Cell::Empty]);
    sheets[0].1.push(vec![Cell::Empty]);
    let preview = run(&sheets, false).await;
    assert_eq!(preview.counts.catalogue, 2);
    assert_eq!(preview.catalogue[0].book.title, "Synthetic board book");
    assert!(preview
        .warnings
        .iter()
        .any(|warning| warning.code == "unmapped_column"));
    assert_eq!(
        preview.catalogue[0].source.raw["Extra source column"].text(),
        "Must remain in provenance"
    );
    assert_eq!(preview.catalogue[0].source.row_number, 3);
}

#[tokio::test]
async fn source_identity_survives_header_reordering_and_a_data_title_is_not_a_header() {
    let original = run(&xlsx::family(), false).await;
    let mut reordered = xlsx::family();
    for row in &mut reordered[1].1 {
        row.reverse();
        row.insert(0, Cell::Empty);
    }
    let preview = run(&reordered, false).await;
    assert_eq!(
        original.history[0].source.key,
        preview.history[0].source.key
    );
    assert_eq!(
        original.history[1].source.key,
        preview.history[1].source.key
    );
    let mut invalid_header = xlsx::family();
    invalid_header[0].1[0][0] = text("Book name");
    invalid_header[0].1[1][0] = text("Title");
    let preview = run(&invalid_header, false).await;
    assert!(!preview.blocking_errors.is_empty());
    assert_eq!(preview.counts.catalogue, 2);
}

#[tokio::test]
async fn dates_unknowns_ratings_and_unsupported_values_are_loss_aware() {
    let mut sheets = xlsx::family();
    sheets[0].1[1][5] = text("maybe");
    sheets[0].1[1][6] = text("Loved");
    sheets[0].1[1][7] = text("Liked");
    sheets[0].1[1][3] = text("not a year");
    sheets[0].1[1][8] = Cell::Boolean(false);
    sheets[1].1[1][4] = text("01/02/2020");
    sheets[1].1[2][4] = text("2020-03");
    let preview = run(&sheets, false).await;
    assert_eq!(preview.catalogue[0].book.owned, None);
    assert_eq!(preview.catalogue[0].book.publication_year, None);
    assert_eq!(preview.catalogue[0].favourite, Some(false));
    assert_eq!(preview.catalogue[0].carly_rating, Some(Rating::Loved));
    assert_eq!(preview.catalogue[0].mom_rating, Some(Rating::Liked));
    assert_eq!(preview.counts.mom_memberships, 1);
    assert_eq!(preview.history[0].source_date.value, None);
    assert_eq!(
        preview.history[1].source_date.value.as_deref(),
        Some("2020-03")
    );
    assert_eq!(preview.history[1].source_date.precision, "month");
    assert!(preview
        .warnings
        .iter()
        .any(|warning| warning.code == "ambiguous_date"));
    assert_eq!(preview.catalogue[0].source.raw["Owned?"].text(), "maybe");
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("synthetic.xlsx");
    xlsx::write(&file, &sheets, false);
    let workbook = xlsx_reader::inspect(&file).unwrap();
    let pool = persistence::open(&directory.path().join("db"))
        .await
        .unwrap();
    let mut draft = ImportReviewDraft::initial("synthetic-token");
    draft.date_order = DateOrder::DayMonthYear;
    let preview = import_preview::preview(&pool, &workbook, draft)
        .await
        .unwrap();
    assert_eq!(
        preview.history[0].source_date.value.as_deref(),
        Some("2020-02-01")
    );
    pool.close().await;
}

#[tokio::test]
async fn missing_and_ambiguous_sheets_and_headers_are_reported_without_guessing() {
    let mut missing_optional = xlsx::family();
    missing_optional.retain(|(name, _)| name != "Dashboard");
    let preview = run(&missing_optional, false).await;
    assert!(preview.blocking_errors.is_empty());
    assert!(preview.missing_sheets.contains(&"Dashboard".into()));
    let mut missing_required = xlsx::family();
    missing_required.retain(|(name, _)| name != CATALOGUE);
    let preview = run(&missing_required, false).await;
    assert!(!preview.blocking_errors.is_empty());
    assert_eq!(preview.counts.catalogue, 0);
    let mut duplicate = xlsx::family();
    duplicate.push((" book catalogue ".into(), duplicate[0].1.clone()));
    assert!(!run(&duplicate, false).await.blocking_errors.is_empty());
    let mut headers = xlsx::family();
    headers[0].1[0][0] = text("Unexpected title name");
    assert!(!run(&headers, false).await.blocking_errors.is_empty());
}

#[tokio::test]
async fn serial_and_partial_dates_retain_original_precision_and_date_system() {
    let mut sheets = xlsx::family();
    sheets[1].1[1][4] = Cell::Number(45000.0);
    sheets[1].1[2][4] = text("2020");
    let preview = run(&sheets, false).await;
    assert_eq!(
        preview.history[0].source_date.value.as_deref(),
        Some("2023-03-15")
    );
    assert_eq!(
        preview.history[0].source_date.raw.as_ref().unwrap().kind,
        "number"
    );
    assert_eq!(preview.history[1].source_date.precision, "year");
    sheets[1].1[1][4] = Cell::Date(0.0);
    let preview = run(&sheets, true).await;
    assert!(preview.date_1904);
    assert_eq!(
        preview.history[0].source_date.value.as_deref(),
        Some("1904-01-01")
    );
    sheets[1].1[1][4] = Cell::Date(60.0);
    let preview = run(&sheets, false).await;
    assert_eq!(preview.history[0].source_date.value, None);
}

#[test]
fn corrupt_and_oversized_workbooks_fail_with_sanitized_errors() {
    assert!(xlsx_reader::inspect_bytes("synthetic.xlsx", b"invalid zip").is_err());
    let oversized = vec![0; xlsx_reader::MAX_FILE_BYTES as usize + 1];
    assert!(xlsx_reader::inspect_bytes("synthetic.xlsx", &oversized).is_err());
}

#[tokio::test]
#[ignore = "Private workbook: run explicitly; no source rows are printed or committed"]
async fn supplied_private_workbook_matches_documented_aggregate_counts() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let file = root
        .join("example")
        .join("Carly's Little Library import.xlsx");
    let workbook = xlsx_reader::inspect(&file).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let preview =
        import_preview::preview(&pool, &workbook, ImportReviewDraft::initial("private-test"))
            .await
            .unwrap();
    assert_eq!(preview.counts.catalogue, 312);
    assert_eq!(preview.counts.historical_loans, 253);
    assert_eq!(preview.counts.title_reviews, 35);
    assert_eq!(preview.counts.owned_yes, 66);
    assert_eq!(preview.counts.currently_borrowed, 3);
    assert_eq!(preview.counts.favourites_yes, 37);
    assert_eq!(preview.counts.mom_memberships, 0);
    assert!(preview.blocking_errors.is_empty());
    pool.close().await;
}

#[test]
#[ignore = "Emits an ignored synthetic fixture for the Windows native-picker smoke test"]
fn emit_desktop_fixture() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let folder = root.join(".local/fixtures");
    std::fs::create_dir_all(&folder).unwrap();
    xlsx::write(&folder.join("import-smoke.xlsx"), &xlsx::family(), false);
}

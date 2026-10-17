use bookbuddy_lib::{
    domain::books::*,
    persistence,
    services::{books, household},
};
use std::time::Instant;

#[tokio::test]
#[ignore = "Emits a synthetic closed database for Windows pagination acceptance"]
async fn emit_paging_fixture() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    household::create(&pool, "Synthetic paging reader")
        .await
        .unwrap();
    let person = household::list(&pool).await.unwrap().people.remove(0).id;
    sqlx::raw_sql("WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<1001) INSERT INTO book (id,title) SELECT printf('00000000-0000-4000-8000-%012d',x), printf('Synthetic paging %04d',x) FROM n;").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO person_book (id,person_id,book_id) SELECT id, ?, id FROM book")
        .bind(&person)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO reading_event (id,book_id,occurred_on) SELECT id,id,'2020-01-02' FROM book",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO reading_event_participant (event_id,person_id) SELECT id,? FROM reading_event",
    )
    .bind(person)
    .execute(&pool)
    .await
    .unwrap();
    let snapshot = persistence::snapshots::snapshot(&pool, directory.path(), "manual")
        .await
        .unwrap();
    pool.close().await;
    let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".local/fixtures");
    std::fs::create_dir_all(&output).unwrap();
    std::fs::copy(snapshot, output.join("paging.sqlite3")).unwrap();
}
#[tokio::test]
#[ignore = "Opt-in synthetic performance profile; outputs timings and aggregate counts only"]
async fn profile_five_thousand_books_and_twenty_thousand_readings() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    household::create(&pool, "Synthetic performance reader")
        .await
        .unwrap();
    let person = household::list(&pool).await.unwrap().people.remove(0).id;
    let mut transaction = pool.begin().await.unwrap();
    sqlx::raw_sql("WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<5000) INSERT INTO book (id,title,authors,owned) SELECT printf('00000000-0000-4000-8000-%012d',x), printf('Synthetic book %05d',x), 'Synthetic author', x%2 FROM n;").execute(&mut *transaction).await.unwrap();
    sqlx::query("INSERT INTO person_book (id,person_id,book_id,reading_status) SELECT id, ?, id, 'read' FROM book").bind(&person).execute(&mut *transaction).await.unwrap();
    sqlx::raw_sql("WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<20000) INSERT INTO reading_event (id,book_id,occurred_on) SELECT printf('10000000-0000-4000-8000-%012d',x), printf('00000000-0000-4000-8000-%012d',1+(x-1)%5000), '2020-01-02' FROM n;").execute(&mut *transaction).await.unwrap();
    sqlx::query("INSERT INTO reading_event_participant (event_id,person_id) SELECT id, ? FROM reading_event").bind(&person).execute(&mut *transaction).await.unwrap();
    transaction.commit().await.unwrap();
    for (name, filter) in [
        ("browse", BookFilter::default()),
        (
            "search",
            BookFilter {
                search: "book 00042".into(),
                ..Default::default()
            },
        ),
        (
            "scoped filter",
            BookFilter {
                person_id: Some(person),
                owned: Filter::Equals(true),
                sort: BookSort::ReadingCount,
                ..Default::default()
            },
        ),
    ] {
        let mut durations = Vec::new();
        let mut count = 0;
        for _ in 0..5 {
            let start = Instant::now();
            let result = books::list(&pool, filter.clone()).await.unwrap();
            durations.push(start.elapsed().as_millis());
            count = result.len();
        }
        durations.sort();
        println!(
            "{name}: median {} ms, max {} ms, {count} results",
            durations[2], durations[4]
        );
        assert!(
            durations[4] < 5000,
            "Investigate a multi-second local query before release"
        );
    }
    let start = Instant::now();
    let events = bookbuddy_lib::services::readings::list(&pool, Default::default())
        .await
        .unwrap();
    println!(
        "activity: {} ms, {} events",
        start.elapsed().as_millis(),
        events.len()
    );
    assert_eq!(events.len(), 20000);
    pool.close().await;
}

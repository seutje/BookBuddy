use bookbuddy_lib::{domain::ErrorCode, persistence, services::household};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row,
};

#[tokio::test]
async fn people_selection_rename_and_everyone_survive_restart() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    assert!(household::list(&pool).await.unwrap().people.is_empty());
    household::create(&pool, "  Reader A  ").await.unwrap();
    let first = household::list(&pool).await.unwrap();
    let first_id = first.people[0].id.clone();
    assert_eq!(first.people[0].display_name, "Reader A");
    assert_eq!(first.current_person_id.as_deref(), Some(first_id.as_str()));
    household::create(&pool, "Reader B").await.unwrap();
    let second = household::list(&pool).await.unwrap();
    assert_ne!(second.current_person_id, first.current_person_id);
    household::rename(&pool, &first_id, "Renamed reader")
        .await
        .unwrap();
    household::select(&pool, Some(&first_id)).await.unwrap();
    pool.close().await;

    let pool = persistence::open(directory.path()).await.unwrap();
    let reopened = household::list(&pool).await.unwrap();
    assert_eq!(
        reopened.current_person_id.as_deref(),
        Some(first_id.as_str())
    );
    assert_eq!(
        reopened
            .people
            .iter()
            .find(|p| p.id == first_id)
            .unwrap()
            .display_name,
        "Renamed reader"
    );
    household::select(&pool, None).await.unwrap();
    pool.close().await;
    let pool = persistence::open(directory.path()).await.unwrap();
    let everyone = household::list(&pool).await.unwrap();
    assert_eq!(everyone.current_person_id, None);
    assert_eq!(everyone.people.len(), 2);
    assert!(!everyone.people.iter().any(|person| person.id == "everyone"));
    pool.close().await;
}

#[tokio::test]
async fn archive_preserves_shared_books_and_other_people_and_clears_selection() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    household::create(&pool, "Reader A").await.unwrap();
    let first_id = household::list(&pool)
        .await
        .unwrap()
        .current_person_id
        .unwrap();
    household::create(&pool, "Reader B").await.unwrap();
    let second_id = household::list(&pool)
        .await
        .unwrap()
        .current_person_id
        .unwrap();
    sqlx::query("INSERT INTO book (id, title, owned) VALUES ('shared', 'Shared book', NULL)")
        .execute(&pool)
        .await
        .unwrap();
    for (id, person, status, rating) in [
        ("membership-a", &first_id, "read", "loved"),
        ("membership-b", &second_id, "want_to_read", "nope"),
    ] {
        sqlx::query("INSERT INTO person_book (id, person_id, book_id, reading_status, rating) VALUES (?, ?, 'shared', ?, ?)")
            .bind(id).bind(person).bind(status).bind(rating).execute(&pool).await.unwrap();
    }
    household::select(&pool, Some(&first_id)).await.unwrap();
    household::archive(&pool, &second_id).await.unwrap();
    assert_eq!(
        household::list(&pool)
            .await
            .unwrap()
            .current_person_id
            .as_deref(),
        Some(first_id.as_str())
    );
    household::archive(&pool, &first_id).await.unwrap();
    let snapshot = household::list(&pool).await.unwrap();
    assert_eq!(snapshot.current_person_id, None);
    assert!(snapshot.people.iter().all(|p| p.archived_at.is_some()));
    assert!(matches!(
        household::select(&pool, Some(&first_id))
            .await
            .unwrap_err()
            .code,
        ErrorCode::NotFound
    ));
    assert!(household::rename(&pool, &first_id, "Cannot edit archive")
        .await
        .is_err());
    let rows = sqlx::query("SELECT reading_status, rating FROM person_book ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get::<String, _>("rating"), "loved");
    assert_eq!(rows[1].get::<String, _>("reading_status"), "want_to_read");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM book")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert!(sqlx::query("DELETE FROM person WHERE id = ?")
        .bind(&first_id)
        .execute(&pool)
        .await
        .is_err());
    pool.close().await;
}

#[tokio::test]
async fn invalid_writes_and_dropped_transactions_leave_existing_data_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    for name in ["", " \t\n", "invalid\0name", &"a".repeat(101)] {
        assert_eq!(
            household::create(&pool, name).await.unwrap_err().code,
            ErrorCode::Validation
        );
    }
    household::create(&pool, "Valid reader").await.unwrap();
    let before = household::list(&pool).await.unwrap();
    for id in ["everyone".to_string(), uuid::Uuid::new_v4().to_string()] {
        assert!(household::rename(&pool, &id, "Replacement").await.is_err());
        assert!(household::archive(&pool, &id).await.is_err());
        assert!(household::select(&pool, Some(&id)).await.is_err());
    }
    let mut transaction = persistence::household::begin(&pool).await.unwrap();
    let transient = uuid::Uuid::new_v4().to_string();
    persistence::household::create(&mut transaction, &transient, "Rollback reader")
        .await
        .unwrap();
    transaction.rollback().await.unwrap();
    let after = household::list(&pool).await.unwrap();
    assert_eq!(after.people.len(), 1);
    assert_eq!(before.current_person_id, after.current_person_id);
    assert_eq!(after.people[0].display_name, "Valid reader");
    pool.close().await;
}

#[tokio::test]
async fn membership_constraints_and_three_valued_ownership_are_enforced() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    household::create(&pool, "Reader").await.unwrap();
    let person = household::list(&pool)
        .await
        .unwrap()
        .current_person_id
        .unwrap();
    sqlx::query("INSERT INTO book (id, title) VALUES ('book', 'Title only')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO person_book (id, person_id, book_id, list_status) VALUES ('membership', ?, 'book', 'wishlist')").bind(&person).execute(&pool).await.unwrap();
    assert!(sqlx::query(
        "INSERT INTO person_book (id, person_id, book_id) VALUES ('duplicate', ?, 'book')"
    )
    .bind(&person)
    .execute(&pool)
    .await
    .is_err());
    assert!(sqlx::query(
        "INSERT INTO person_book (id, person_id, book_id) VALUES ('missing', 'missing', 'book')"
    )
    .execute(&pool)
    .await
    .is_err());
    assert!(sqlx::query("UPDATE person_book SET rating = '0'")
        .execute(&pool)
        .await
        .is_err());
    assert!(sqlx::query("UPDATE book SET owned = 2")
        .execute(&pool)
        .await
        .is_err());
    let unknown: Option<i64> = sqlx::query_scalar("SELECT owned FROM book")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(unknown, None);
    for value in [0_i64, 1] {
        sqlx::query("UPDATE book SET owned = ?")
            .bind(value)
            .execute(&pool)
            .await
            .unwrap();
        let owned: Option<i64> = sqlx::query_scalar("SELECT owned FROM book")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(owned, Some(value));
        let list: String = sqlx::query_scalar("SELECT list_status FROM person_book")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(list, "wishlist");
    }
    assert!(sqlx::query("DELETE FROM book")
        .execute(&pool)
        .await
        .is_err());
    pool.close().await;
}

#[tokio::test]
async fn upgrade_and_failed_migration_preserve_existing_data() {
    let directory = tempfile::tempdir().unwrap();
    let old_migrations = tempfile::tempdir().unwrap();
    std::fs::write(
        old_migrations.path().join("0001_database_identity.sql"),
        include_str!("../migrations/0001_database_identity.sql"),
    )
    .unwrap();
    let options = SqliteConnectOptions::new()
        .filename(directory.path().join("bookbuddy.sqlite3"))
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .connect_with(options.clone())
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(old_migrations.path())
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE upgrade_probe (value TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO upgrade_probe VALUES ('keep')")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    let pool = persistence::open(directory.path()).await.unwrap();
    let value: String = sqlx::query_scalar("SELECT value FROM upgrade_probe")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(value, "keep");
    household::create(&pool, "Preserved reader").await.unwrap();
    sqlx::query("UPDATE _sqlx_migrations SET checksum = X'00' WHERE version = 1")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    assert!(persistence::open(directory.path()).await.is_err());
    let raw = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .unwrap();
    let name: String = sqlx::query_scalar("SELECT display_name FROM person")
        .fetch_one(&raw)
        .await
        .unwrap();
    assert_eq!(name, "Preserved reader");
    raw.close().await;
}

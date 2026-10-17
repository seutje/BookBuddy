use bookbuddy_lib::persistence;
use sqlx::Row;

#[tokio::test]
async fn database_is_local_migrated_and_survives_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let pool = persistence::open(directory.path()).await.unwrap();
    let mut connections = Vec::new();
    for _ in 0..4 {
        let mut connection = pool.acquire().await.unwrap();
        let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        assert_eq!(foreign_keys, 1);
        connections.push(connection);
    }
    drop(connections);
    let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(mode, "wal");
    let identity: i64 = sqlx::query_scalar("PRAGMA application_id")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(identity, 1111643460);
    sqlx::query("CREATE TABLE persistence_probe (value TEXT NOT NULL)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO persistence_probe VALUES ('preserved')")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let reopened = persistence::open(directory.path()).await.unwrap();
    let row = sqlx::query("SELECT value FROM persistence_probe")
        .fetch_one(&reopened)
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("value"), "preserved");
    let migrations: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = 1")
            .fetch_one(&reopened)
            .await
            .unwrap();
    assert_eq!(migrations, 2);
    reopened.close().await;
}

#[tokio::test]
async fn invalid_existing_database_is_not_reset() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("bookbuddy.sqlite3");
    let original = b"invalid database: do not replace";
    std::fs::write(&file, original).unwrap();
    assert!(persistence::open(directory.path()).await.is_err());
    assert_eq!(std::fs::read(file).unwrap(), original);
}

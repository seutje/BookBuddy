//! SQLite access stays in this module, including connection configuration.
pub mod backups;
pub mod books;
pub mod custom_fields;
pub mod household;
pub mod import;
pub mod readings;
pub mod snapshots;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::{path::Path, time::Duration};

/// Windows sharing denial is released by the OS even after process termination.
/// Keep this handle for the application's lifetime, including during restore.
pub fn lock_directory(directory: &Path) -> std::io::Result<std::fs::File> {
    std::fs::create_dir_all(directory)?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }
    options.open(directory.join("bookbuddy.lock"))
}

pub async fn open(directory: &Path) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(directory)?;
    let options = SqliteConnectOptions::new()
        .filename(directory.join("bookbuddy.sqlite3"))
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;
    if let Err(error) = sqlx::migrate!("./migrations").run(&pool).await {
        pool.close().await;
        return Err(Box::new(error));
    }
    Ok(pool)
}

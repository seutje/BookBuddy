use crate::domain::AppError;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};

/// VACUUM INTO produces a transactionally consistent snapshot, including WAL data.
/// The destination is always a new file; a live SQLite database is never copied.
pub async fn snapshot(
    pool: &SqlitePool,
    directory: &Path,
    purpose: &str,
) -> Result<PathBuf, AppError> {
    if !matches!(
        purpose,
        "pre-import" | "pre-restore" | "manual" | "automatic"
    ) {
        return Err(AppError::validation("Invalid snapshot purpose."));
    }
    std::fs::create_dir_all(directory).map_err(|_| {
        AppError::validation(
            "The local recovery-backup folder could not be created. Check folder access and free disk space.",
        )
    })?;
    let destination = directory.join(format!("{purpose}-{}.sqlite3", uuid::Uuid::new_v4()));
    sqlx::query("VACUUM INTO ?")
        .bind(destination.to_str().ok_or_else(|| {
            AppError::validation("The backup destination could not be represented safely.")
        })?)
        .execute(pool)
        .await?;
    Ok(destination)
}

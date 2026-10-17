//! All raw SQLite calls are confined to the atomic online-backup operation below.
use crate::{
    domain::{backups::*, AppError},
    services::xlsx_reader::hash,
};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Connection, SqliteConnection, SqlitePool,
};
use std::{path::Path, str::FromStr};

pub async fn counts(pool: &SqlitePool) -> Result<Counts, AppError> {
    let (books, people, memberships, fields, readings, historical_loans, import_batches, source_rows, title_tasks) = sqlx::query_as::<_, (i64,i64,i64,i64,i64,i64,i64,i64,i64)>("SELECT (SELECT COUNT(*) FROM book), (SELECT COUNT(*) FROM person), (SELECT COUNT(*) FROM person_book), (SELECT COUNT(*) FROM custom_field_definition), (SELECT COUNT(*) FROM reading_event), (SELECT COUNT(*) FROM library_borrow_event), (SELECT COUNT(*) FROM import_batch), (SELECT COUNT(*) FROM import_source_row), (SELECT COUNT(*) FROM book_title_verification)").fetch_one(pool).await?;
    Ok(Counts {
        books,
        people,
        memberships,
        fields,
        readings,
        historical_loans,
        import_batches,
        source_rows,
        title_tasks,
    })
}
pub async fn settings(pool: &SqlitePool) -> Result<BackupSettings, AppError> {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT value_json FROM app_settings WHERE key = 'backup_settings'")
            .fetch_optional(pool)
            .await?;
    let settings = raw
        .map(|raw| serde_json::from_str::<BackupSettings>(&raw))
        .transpose()
        .map_err(|_| AppError::validation("Backup settings need review."))?
        .unwrap_or_default();
    settings.validate()?;
    Ok(settings)
}
pub async fn save_settings(pool: &SqlitePool, settings: &BackupSettings) -> Result<(), AppError> {
    settings.validate()?;
    sqlx::query("INSERT INTO app_settings (key, value_json) VALUES ('backup_settings', ?) ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json").bind(serde_json::to_string(settings).map_err(|_| AppError::validation("Invalid backup settings."))?).execute(pool).await?;
    Ok(())
}
pub async fn state_hash(pool: &SqlitePool) -> Result<String, AppError> {
    // A consistent snapshot hashes every record, including settings and provenance.
    let mut transaction = pool.begin().await?;
    let _: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_schema")
        .fetch_one(&mut *transaction)
        .await?;
    let bytes = transaction.serialize(None).await?.to_vec();
    transaction.rollback().await?;
    Ok(hash(&bytes))
}
type SchemaRow = (String, String, String, Option<String>);
async fn schema(pool: &SqlitePool) -> Result<Vec<SchemaRow>, AppError> {
    Ok(sqlx::query_as("SELECT type, name, tbl_name, sql FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' AND name != '_sqlx_migrations' ORDER BY type, name").fetch_all(pool).await?)
}
pub async fn validate_snapshot(path: &Path, version: i64) -> Result<SqlitePool, AppError> {
    let migrator = sqlx::migrate!("./migrations");
    if !migrator
        .iter()
        .any(|migration| migration.version == version)
    {
        return Err(AppError::validation(
            "This backup schema version is not supported. Use a compatible BookBuddy version.",
        ));
    }
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .read_only(true)
                .foreign_keys(true)
                .pragma("trusted_schema", "OFF"),
        )
        .await?;
    let result = async {
        let application_id: i64 = sqlx::query_scalar("PRAGMA application_id").fetch_one(&pool).await?;
        if application_id != 1111643460 { return Err(AppError::validation("This is not a BookBuddy database.")); }
        let integrity: Vec<String> = sqlx::query_scalar("PRAGMA integrity_check").fetch_all(&pool).await?;
        if integrity != ["ok"] { return Err(AppError::validation("The backup database failed its integrity check.")); }
        let migration_table: Option<String> = sqlx::query_scalar("SELECT type FROM sqlite_schema WHERE name = '_sqlx_migrations'").fetch_optional(&pool).await?;
        if migration_table.as_deref() != Some("table") { return Err(AppError::validation("The backup migration history is missing.")); }
        let saved: Vec<(i64, bool, Vec<u8>)> = sqlx::query_as("SELECT version, success, checksum FROM _sqlx_migrations ORDER BY version").fetch_all(&pool).await?;
        let expected: Vec<_> = migrator.iter().filter(|migration| migration.version <= version).collect();
        if saved.len() != expected.len() || saved.iter().zip(&expected).any(|((v, success, checksum), migration)| *v != migration.version || !success || checksum.as_slice() != migration.checksum.as_ref()) { return Err(AppError::validation("The backup migration history is incompatible or incomplete.")); }
        let canonical = SqlitePoolOptions::new().max_connections(1).connect_with(SqliteConnectOptions::from_str("sqlite::memory:")?).await?;
        for migration in &expected { sqlx::raw_sql(&migration.sql).execute(&canonical).await?; }
        let matches = schema(&pool).await? == schema(&canonical).await?;
        canonical.close().await;
        if !matches { return Err(AppError::validation("The backup schema differs from the supported BookBuddy schema.")); }
        if !sqlx::query("PRAGMA foreign_key_check").fetch_all(&pool).await?.is_empty() { return Err(AppError::validation("The backup contains broken record links.")); }
        let covers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM book WHERE cover_path IS NOT NULL").fetch_one(&pool).await?;
        if covers != 0 { return Err(AppError::validation("This version cannot restore cover assets. Keep the original backup and use a compatible version.")); }
        Ok(())
    }.await;
    if let Err(error) = result {
        pool.close().await;
        return Err(error);
    }
    Ok(pool)
}

/// SQLite holds the destination write transaction throughout the backup. Finishing
/// an incomplete backup rolls it back, including after a process interruption.
/// Callers must exclude other application operations, then close/reopen the pool
/// after success to discard prepared statements from the previous schema.
pub async fn replace_from_snapshot(
    pool: &SqlitePool,
    source: &Path,
    stop_after_pages: Option<i32>,
) -> Result<(), AppError> {
    let mut source = SqliteConnection::connect_with(
        &SqliteConnectOptions::new().filename(source).read_only(true),
    )
    .await?;
    let mut destination = pool.acquire().await?;
    let result = {
        let mut destination_handle = destination.lock_handle().await?;
        let mut source_handle = source.lock_handle().await?;
        // SAFETY: both SQLx worker handles are exclusively locked for the whole
        // FFI lifetime. The static main names are NUL-terminated; no pointers
        // escape this scope. finish always runs before either guard is dropped.
        unsafe {
            let backup = libsqlite3_sys::sqlite3_backup_init(
                destination_handle.as_raw_handle().as_ptr(),
                c"main".as_ptr(),
                source_handle.as_raw_handle().as_ptr(),
                c"main".as_ptr(),
            );
            if backup.is_null() {
                Err(AppError::validation("Restore could not obtain exclusive database access. Close other BookBuddy windows and retry."))
            } else {
                let step =
                    libsqlite3_sys::sqlite3_backup_step(backup, stop_after_pages.unwrap_or(-1));
                let finish = libsqlite3_sys::sqlite3_backup_finish(backup);
                if step == libsqlite3_sys::SQLITE_DONE && finish == libsqlite3_sys::SQLITE_OK {
                    Ok(())
                } else {
                    Err(AppError::validation("Restore did not finish. SQLite rolled back the replacement; your existing library is preserved."))
                }
            }
        }
    };
    drop(source);
    result
}

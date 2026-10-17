//! Household operations and transaction orchestration.
pub mod backups;
pub mod books;
pub mod custom_fields;
pub mod household;
pub mod import_commit;
pub mod import_history;
pub mod import_preview;
pub mod readings;
pub mod xlsx_reader;

/// Test runs may select a UUID leaf only within the isolated debug data folder.
/// Release builds never accept a data-location override.
pub fn data_directory(
    base: std::path::PathBuf,
    development: bool,
    test_run: Option<&str>,
) -> Result<std::path::PathBuf, crate::domain::AppError> {
    if !development {
        return Ok(base);
    }
    let base = base.join("development");
    match test_run {
        Some(id) => {
            let id = uuid::Uuid::parse_str(id)
                .map_err(|_| crate::domain::AppError::validation("Invalid test run ID."))?;
            Ok(base.join("smoke").join(id.to_string()))
        }
        None => Ok(base),
    }
}

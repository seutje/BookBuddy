//! Household operations and transaction orchestration.
pub mod backups;
pub mod books;
pub mod custom_fields;
pub mod household;
pub mod import_commit;
pub mod import_history;
pub mod import_preview;
pub mod readings;
pub mod updates;
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

/// Installed-release acceptance tests may select only a fresh UUID leaf under
/// the dedicated acceptance root. No argument can name a household directory.
pub fn launch_directory(
    base: std::path::PathBuf,
    development: bool,
    test_run: Option<&str>,
    arguments: &[String],
) -> Result<std::path::PathBuf, crate::domain::AppError> {
    let acceptance: Vec<_> = arguments
        .iter()
        .filter_map(|argument| argument.strip_prefix("--acceptance-run="))
        .collect();
    if acceptance.len() > 1 {
        return Err(crate::domain::AppError::validation(
            "Use one acceptance run ID.",
        ));
    }
    if let Some(id) = acceptance.first() {
        let id = uuid::Uuid::parse_str(id)
            .map_err(|_| crate::domain::AppError::validation("Invalid acceptance run ID."))?;
        return Ok(base.join("acceptance").join(id.to_string()));
    }
    data_directory(base, development, test_run)
}

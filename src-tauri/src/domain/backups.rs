use super::AppError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    pub schema_version: i64,
    pub app_version: String,
    pub created_at: String,
    pub database_sha256: String,
    pub database_bytes: u64,
    pub cover_assets: Vec<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub books: i64,
    pub people: i64,
    pub memberships: i64,
    pub fields: i64,
    pub readings: i64,
    pub historical_loans: i64,
    pub import_batches: i64,
    pub source_rows: i64,
    pub title_tasks: i64,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreview {
    pub token: String,
    pub source_name: String,
    pub manifest: Manifest,
    pub backup_counts: Counts,
    pub current_counts: Counts,
    pub review_hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupSettings {
    pub enabled: bool,
    pub keep: u32,
}
impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            keep: 5,
        }
    }
}
impl BackupSettings {
    pub fn validate(&self) -> Result<(), AppError> {
        if !(1..=20).contains(&self.keep) {
            return Err(AppError::validation(
                "Keep between 1 and 20 automatic backups.",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatus {
    pub settings: BackupSettings,
    pub automatic_count: usize,
    pub automatic_problem: bool,
    pub backup_directory: String,
}

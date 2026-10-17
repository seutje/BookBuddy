//! Domain types and validation, independent of the presentation layer.
pub mod books;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Validation,
    NotFound,
    Storage,
    Startup,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
}

impl AppError {
    pub fn validation(message: &str) -> Self {
        Self {
            code: ErrorCode::Validation,
            message: message.into(),
        }
    }

    pub fn not_found() -> Self {
        Self {
            code: ErrorCode::NotFound,
            message: "That active person could not be found. Refresh and try again.".into(),
        }
    }

    pub fn startup() -> Self {
        Self { code: ErrorCode::Startup, message: "BookBuddy could not open or migrate its local database. Existing data has been preserved. Close other instances and retry. If the problem continues, keep the data folder for recovery; do not delete it.".into() }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(_: sqlx::Error) -> Self {
        Self {
            code: ErrorCode::Storage,
            message:
                "The local database operation failed. Your changes were not saved. Please retry."
                    .into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: String,
    pub display_name: String,
    pub archived_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Household {
    pub people: Vec<Person>,
    pub current_person_id: Option<String>,
}

pub fn person_name(raw: &str) -> Result<String, AppError> {
    let name = raw.trim();
    if name.is_empty() || name.chars().count() > 100 || name.chars().any(char::is_control) {
        return Err(AppError::validation(
            "Enter a name of 1–100 characters without control characters.",
        ));
    }
    Ok(name.to_owned())
}

pub fn durable_id(raw: &str) -> Result<(), AppError> {
    uuid::Uuid::parse_str(raw).map_err(|_| AppError::validation("Invalid record ID."))?;
    Ok(())
}

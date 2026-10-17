use super::AppError;
use serde::{Deserialize, Serialize};

macro_rules! choice {
    ($name:ident { $($variant:ident),+ }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, sqlx::Type)]
        #[serde(rename_all = "snake_case")]
        #[sqlx(type_name = "TEXT", rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}
choice!(ReadingStatus {
    WantToRead,
    OnHold,
    CurrentlyBorrowed,
    Read,
    Reading,
    DidNotFinish
});
choice!(Rating {
    Loved,
    Liked,
    Meh,
    Nope
});
choice!(ListStatus {
    Library,
    Wishlist,
    Borrowed
});

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BookDraft {
    pub title: String,
    pub subtitle: Option<String>,
    pub authors: Option<String>,
    pub isbn13: Option<String>,
    pub publisher: Option<String>,
    pub publication_year: Option<i64>,
    pub language: Option<String>,
    pub series: Option<String>,
    pub volume: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub owned: Option<bool>,
}

impl BookDraft {
    pub fn validate(mut self) -> Result<Self, AppError> {
        self.title = self.title.trim().to_string();
        if self.title.is_empty()
            || self.title.chars().count() > 1000
            || self.title.chars().any(char::is_control)
        {
            return Err(AppError::validation(
                "Enter a title of 1–1000 characters without control characters.",
            ));
        }
        for value in [
            &self.subtitle,
            &self.authors,
            &self.publisher,
            &self.language,
            &self.series,
            &self.volume,
        ] {
            if value.as_ref().is_some_and(|value| {
                value.chars().count() > 5000 || value.chars().any(char::is_control)
            }) {
                return Err(AppError::validation("Bibliographic fields must be at most 5000 characters without control characters."));
            }
        }
        validate_notes(&self.notes)?;
        validate_notes(&self.description)?;
        if self
            .publication_year
            .is_some_and(|year| !(1..=3000).contains(&year))
        {
            return Err(AppError::validation(
                "Publication year must be between 1 and 3000, or left unknown.",
            ));
        }
        if let Some(isbn) = &self.isbn13 {
            let compact: String = isbn
                .chars()
                .filter(|c| *c != '-' && !c.is_whitespace())
                .collect();
            if compact.is_empty() {
                self.isbn13 = None;
            } else {
                if !(compact.starts_with("978") || compact.starts_with("979"))
                    || compact.len() != 13
                    || !compact.bytes().all(|b| b.is_ascii_digit())
                    || compact
                        .bytes()
                        .enumerate()
                        .map(|(i, digit)| u32::from(digit - b'0') * if i % 2 == 0 { 1 } else { 3 })
                        .sum::<u32>()
                        % 10
                        != 0
                {
                    return Err(AppError::validation(
                        "Enter a valid ISBN-13 with its check digit, or leave it blank.",
                    ));
                }
                self.isbn13 = Some(compact);
            }
        }
        Ok(self)
    }
}

pub fn validate_notes(value: &Option<String>) -> Result<(), AppError> {
    if value
        .as_ref()
        .is_some_and(|value| value.len() > 100_000 || value.contains('\0'))
    {
        return Err(AppError::validation(
            "Notes and descriptions must be at most 100000 bytes without null characters.",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub id: String,
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub data: BookDraft,
    pub created_at: String,
    pub updated_at: String,
    pub read_count: i64,
    pub last_read_on: Option<String>,
    #[sqlx(rename = "has_title_verification")]
    pub needs_title_verification: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PersonalDraft {
    pub reading_status: Option<ReadingStatus>,
    pub rating: Option<Rating>,
    pub list_status: Option<ListStatus>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Membership {
    pub id: String,
    pub person_id: String,
    pub book_id: String,
    pub person_name: String,
    pub archived_at: Option<String>,
    pub reading_status: Option<ReadingStatus>,
    pub rating: Option<Rating>,
    pub list_status: Option<ListStatus>,
    pub notes: Option<String>,
    pub read_count: i64,
    pub last_read_on: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookDetail {
    pub book: Book,
    pub memberships: Vec<Membership>,
    pub row_fields: Vec<super::custom_fields::CustomDisplay>,
    #[serde(skip)]
    pub custom_search: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(tag = "mode", content = "value", rename_all = "snake_case")]
pub enum Filter<T> {
    #[default]
    Any,
    Unknown,
    Equals(T),
}

impl<T> Filter<T> {
    pub fn matches(&self, value: Option<&T>) -> bool
    where
        T: PartialEq,
    {
        match self {
            Self::Any => true,
            Self::Unknown => value.is_none(),
            Self::Equals(expected) => value == Some(expected),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BookSort {
    #[default]
    RecentlyAdded,
    Title,
    Author,
    Rating,
    MostRecentlyRead,
    ReadingCount,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BookFilter {
    pub person_id: Option<String>,
    #[serde(default)]
    pub search: String,
    #[serde(default)]
    pub reading_status: Filter<ReadingStatus>,
    #[serde(default)]
    pub rating: Filter<Rating>,
    #[serde(default)]
    pub list_status: Filter<ListStatus>,
    #[serde(default)]
    pub owned: Filter<bool>,
    #[serde(default)]
    pub sort: BookSort,
    #[serde(default)]
    pub custom_filters: Vec<super::custom_fields::CustomFilter>,
    pub has_readings: Option<bool>,
    pub read_from: Option<String>,
    pub read_to: Option<String>,
    pub needs_title_verification: Option<bool>,
}

pub fn normalized(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

use super::{books::validate_notes, custom_fields::valid_date, durable_id, AppError};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadingDraft {
    pub book_id: String,
    pub occurred_on: String,
    pub participant_ids: Vec<String>,
    pub note: Option<String>,
    pub set_read: bool,
}
impl ReadingDraft {
    pub fn validate(&self, today: &str) -> Result<(), AppError> {
        durable_id(&self.book_id)?;
        if !valid_date(&self.occurred_on) || self.occurred_on.as_str() > today {
            return Err(AppError::validation(
                "Choose a valid local calendar date today or in the past.",
            ));
        }
        if self.participant_ids.is_empty()
            || self.participant_ids.len() > 100
            || self.participant_ids.iter().collect::<HashSet<_>>().len()
                != self.participant_ids.len()
        {
            return Err(AppError::validation(
                "Choose 1–100 distinct participants for this reading.",
            ));
        }
        for id in &self.participant_ids {
            durable_id(id)?;
        }
        validate_notes(&self.note)
    }
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    pub person_id: String,
    pub display_name: String,
    pub archived_at: Option<String>,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ReadingEvent {
    pub id: String,
    pub book_id: String,
    pub title: String,
    pub occurred_on: String,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[sqlx(skip)]
    pub participants: Vec<Participant>,
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadingFilter {
    pub person_id: Option<String>,
    pub book_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}
impl ReadingFilter {
    pub fn validate(&self) -> Result<(), AppError> {
        if let Some(id) = &self.person_id {
            durable_id(id)?;
        }
        if let Some(id) = &self.book_id {
            durable_id(id)?;
        }
        if self.from.as_ref().is_some_and(|date| !valid_date(date))
            || self.to.as_ref().is_some_and(|date| !valid_date(date))
            || matches!((&self.from, &self.to), (Some(from), Some(to)) if from > to)
        {
            return Err(AppError::validation("Choose a valid date range."));
        }
        Ok(())
    }
}
pub fn calendar_date_at(now: time::OffsetDateTime) -> String {
    now.date().to_string()
}
pub fn local_today() -> Result<String, AppError> {
    time::OffsetDateTime::now_local().map(calendar_date_at).map_err(|_| AppError::validation("The Windows local date could not be determined. Check the computer's date and timezone."))
}

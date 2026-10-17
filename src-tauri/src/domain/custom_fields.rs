use super::{books::normalized, durable_id, AppError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum Scope {
    Book,
    PersonBook,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum Kind {
    Boolean,
    Select,
    Number,
    Date,
    Text,
    Rating,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FieldSettings {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub unit: Option<String>,
    #[serde(default)]
    pub multiline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FieldOption {
    pub id: String,
    pub label: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FieldDraft {
    pub scope: Scope,
    pub kind: Kind,
    pub label: String,
    pub description: String,
    pub settings: FieldSettings,
    pub default_value: Option<Value>,
    pub options: Vec<FieldOption>,
    pub order_index: i64,
    pub active: bool,
    pub show_detail: bool,
    pub show_in_rows: bool,
    pub show_in_filters: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDefinition {
    pub id: String,
    #[serde(flatten)]
    pub draft: FieldDraft,
    pub revision: i64,
    pub value_count: i64,
}

impl FieldDraft {
    pub fn validate(mut self) -> Result<Self, AppError> {
        self.label = super::person_name(&self.label)?;
        if self.description.len() > 10000
            || self.description.contains('\0')
            || !(0..=1_000_000).contains(&self.order_index)
            || self.options.len() > 200
        {
            return Err(AppError::validation(
                "Field description, order or choice count is too large.",
            ));
        }
        if self.settings.min.is_some_and(|n| !n.is_finite())
            || self.settings.max.is_some_and(|n| !n.is_finite())
            || matches!((self.settings.min, self.settings.max), (Some(min), Some(max)) if min > max)
        {
            return Err(AppError::validation("Set a valid minimum and maximum."));
        }
        if self
            .settings
            .unit
            .as_ref()
            .is_some_and(|unit| unit.len() > 100 || unit.chars().any(char::is_control))
        {
            return Err(AppError::validation(
                "Unit labels must be at most 100 bytes without control characters.",
            ));
        }
        if !matches!(self.kind, Kind::Number | Kind::Rating)
            && (self.settings.min.is_some()
                || self.settings.max.is_some()
                || self.settings.unit.is_some())
        {
            return Err(AppError::validation(
                "Only number and rating fields have numeric settings.",
            ));
        }
        if self.settings.multiline && self.kind != Kind::Text {
            return Err(AppError::validation("Only text fields can be multiline."));
        }
        if self.kind == Kind::Rating {
            let min = self.settings.min.unwrap_or(1.0);
            let max = self.settings.max.unwrap_or(5.0);
            if min.fract() != 0.0 || max.fract() != 0.0 || min > max {
                return Err(AppError::validation(
                    "Rating ranges require whole-number bounds.",
                ));
            }
        }
        let mut ids = HashSet::new();
        let mut labels = HashSet::new();
        for option in &mut self.options {
            durable_id(&option.id)?;
            option.label = super::person_name(&option.label)?;
            if !ids.insert(option.id.clone())
                || (option.active && !labels.insert(normalized(&option.label)))
            {
                return Err(AppError::validation(
                    "Choices require distinct IDs and active labels.",
                ));
            }
            if option.active && self.kind != Kind::Select {
                return Err(AppError::validation(
                    "Only select fields have active choices.",
                ));
            }
        }
        if let Some(value) = &self.default_value {
            self.validate_value(value, false)?;
        }
        Ok(self)
    }

    pub fn validate_value(
        &self,
        value: &Value,
        allow_retired_option: bool,
    ) -> Result<(), AppError> {
        let valid = match self.kind {
            Kind::Boolean => value.is_boolean(),
            Kind::Select => value.as_str().is_some_and(|id| {
                self.options
                    .iter()
                    .any(|option| option.id == id && (option.active || allow_retired_option))
            }),
            Kind::Number | Kind::Rating => value.as_f64().is_some_and(|number| {
                number.is_finite()
                    && number
                        >= self.settings.min.unwrap_or(if self.kind == Kind::Rating {
                            1.0
                        } else {
                            f64::MIN
                        })
                    && number
                        <= self.settings.max.unwrap_or(if self.kind == Kind::Rating {
                            5.0
                        } else {
                            f64::MAX
                        })
                    && (self.kind != Kind::Rating || number.fract() == 0.0)
            }),
            Kind::Date => value.as_str().is_some_and(valid_date),
            Kind::Text => value.as_str().is_some_and(|text| {
                text.len() <= 100000
                    && !text.contains('\0')
                    && (self.settings.multiline || !text.chars().any(char::is_control))
            }),
        };
        if !valid {
            return Err(AppError::validation(
                "The value does not match this field's type, choices or range.",
            ));
        }
        Ok(())
    }
}

pub fn valid_date(raw: &str) -> bool {
    raw.len() == 10
        && time::Date::parse(
            raw,
            time::macros::format_description!("[year]-[month]-[day]"),
        )
        .is_ok_and(|date| date.year() > 0 && date.to_string() == raw)
}

pub fn canonical_value(draft: &FieldDraft, value: &Value) -> Result<String, AppError> {
    if matches!(draft.kind, Kind::Number | Kind::Rating) {
        serde_json::to_string(
            &value
                .as_f64()
                .ok_or_else(|| AppError::validation("Expected a number."))?,
        )
        .map_err(|_| AppError::validation("Invalid number."))
    } else {
        Ok(value.to_string())
    }
}

pub fn display_value(draft: &FieldDraft, value: Option<&Value>) -> String {
    let Some(value) = value else {
        return "Unknown".into();
    };
    match draft.kind {
        Kind::Boolean => {
            if value.as_bool() == Some(true) {
                "Yes".into()
            } else {
                "No".into()
            }
        }
        Kind::Select => draft
            .options
            .iter()
            .find(|option| Some(option.id.as_str()) == value.as_str())
            .map(|option| {
                format!(
                    "{}{}",
                    option.label,
                    if option.active { "" } else { " (retired)" }
                )
            })
            .unwrap_or_else(|| "Unknown choice".into()),
        Kind::Number | Kind::Rating => format!(
            "{}{}",
            value,
            draft
                .settings
                .unit
                .as_ref()
                .map(|unit| format!(" {unit}"))
                .unwrap_or_default()
        ),
        Kind::Text | Kind::Date => value.as_str().unwrap_or("").to_string(),
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomDisplay {
    pub field_id: String,
    pub label: String,
    pub person_name: Option<String>,
    pub display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case", deny_unknown_fields)]
pub enum ValueTarget {
    Book {
        #[serde(rename = "bookId")]
        book_id: String,
    },
    PersonBook {
        #[serde(rename = "personId")]
        person_id: String,
        #[serde(rename = "bookId")]
        book_id: String,
    },
}
impl ValueTarget {
    pub fn validate(&self) -> Result<(), AppError> {
        match self {
            Self::Book { book_id } => durable_id(book_id),
            Self::PersonBook { person_id, book_id } => {
                durable_id(person_id)?;
                durable_id(book_id)
            }
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldValue {
    pub definition: FieldDefinition,
    pub value: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePreview {
    pub token: String,
    pub requires_confirmation: bool,
    pub cleared_value_count: i64,
    pub saved_value_count: i64,
    pub warnings: Vec<String>,
}

pub fn preview(old: &FieldDefinition, draft: &FieldDraft) -> Result<ChangePreview, AppError> {
    if old.draft.scope != draft.scope && old.value_count > 0 {
        return Err(AppError::validation(
            "Scope cannot change while values exist. Create a new field instead.",
        ));
    }
    let tightened = match draft.kind {
        Kind::Number | Kind::Rating => {
            let fallback_min = if draft.kind == Kind::Rating {
                1.0
            } else {
                f64::MIN
            };
            let fallback_max = if draft.kind == Kind::Rating {
                5.0
            } else {
                f64::MAX
            };
            draft.settings.min.unwrap_or(fallback_min)
                > old.draft.settings.min.unwrap_or(fallback_min)
                || draft.settings.max.unwrap_or(fallback_max)
                    < old.draft.settings.max.unwrap_or(fallback_max)
        }
        Kind::Text => old.draft.settings.multiline && !draft.settings.multiline,
        _ => false,
    };
    let clears = old.value_count > 0 && (old.draft.kind != draft.kind || tightened);
    let retires = old.draft.options.iter().any(|old| {
        old.active
            && !draft
                .options
                .iter()
                .any(|new| new.id == old.id && new.active)
    });
    let hides = old.draft.active && !draft.active;
    let mut warnings = Vec::new();
    if clears {
        warnings.push(format!("This type or validation-setting change clears {} saved values. No automatic conversion is performed.", old.value_count));
    }
    if retires {
        warnings.push("Retired choices remain visible on saved records but cannot be selected for new values.".into());
    }
    if hides {
        warnings.push(format!(
            "This hides the field. Its {} saved values are preserved for reactivation.",
            old.value_count
        ));
    }
    let payload = serde_json::to_vec(&(old.id.as_str(), old.revision, old.value_count, draft))
        .map_err(|_| AppError::validation("Invalid field change."))?;
    let token = format!("{:x}", Sha256::digest(payload));
    Ok(ChangePreview {
        token,
        requires_confirmation: clears || retires || hides,
        cleared_value_count: if clears { old.value_count } else { 0 },
        saved_value_count: old.value_count,
        warnings,
    })
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "mode", content = "value", rename_all = "snake_case")]
pub enum CustomMatch {
    Unknown,
    Equals(Value),
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CustomFilter {
    pub field_id: String,
    pub condition: CustomMatch,
}

pub fn resolve_filters<'a>(
    definitions: &'a [FieldDefinition],
    filters: &'a [CustomFilter],
) -> Result<Vec<(&'a FieldDefinition, &'a CustomFilter)>, AppError> {
    if filters.len() > 30 {
        return Err(AppError::validation("Use at most 30 custom field filters."));
    }
    filters
        .iter()
        .map(|filter| {
            durable_id(&filter.field_id)?;
            let definition = definitions
                .iter()
                .find(|definition| definition.id == filter.field_id && definition.draft.active)
                .ok_or_else(|| {
                    AppError::validation("A custom filter refers to a missing or hidden field.")
                })?;
            if let CustomMatch::Equals(value) = &filter.condition {
                definition.draft.validate_value(value, true)?;
            }
            Ok((definition, filter))
        })
        .collect()
}

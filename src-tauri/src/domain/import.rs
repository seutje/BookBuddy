use super::{
    books::{Book, BookDraft, Rating, ReadingStatus},
    AppError,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const CATALOGUE: &str = "Book Catalogue";
pub const HISTORY: &str = "Library History";
pub const TITLES: &str = "Check Titles";
pub const SHEETS: [&str; 5] = [CATALOGUE, HISTORY, TITLES, "Dashboard", "How to Use"];
pub const CATALOGUE_HEADERS: [&str; 20] = [
    "Title",
    "Author",
    "Format",
    "Published",
    "Reading Status",
    "Owned?",
    "Carly Rating",
    "Mom Rating",
    "Favourite?",
    "Buy Status",
    "Borrow Count",
    "First Library Date",
    "Latest Library Date",
    "Date Type",
    "Reading Effort",
    "Age Range",
    "Renewed to Death?",
    "Notes",
    "Source / Original Title",
    "Check Title?",
];
pub const HISTORY_HEADERS: [&str; 7] = [
    "Title",
    "Author",
    "Format",
    "Publication Year",
    "Approximate Borrow Date",
    "Actual Date Type",
    "Source PDF",
];
pub const TITLE_HEADERS: [&str; 5] = [
    "Working Title",
    "Author",
    "Exactly as PDF shows",
    "Reason",
    "Confirmed Full Title (edit here)",
];
pub fn expected_headers(sheet: &str) -> &'static [&'static str] {
    match sheet {
        CATALOGUE => &CATALOGUE_HEADERS,
        HISTORY => &HISTORY_HEADERS,
        TITLES => &TITLE_HEADERS,
        _ => &[],
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawCell {
    pub kind: String,
    pub value: Value,
    pub calendar_date: Option<String>,
    pub formula: Option<String>,
}
impl RawCell {
    pub fn text(&self) -> String {
        match &self.value {
            Value::String(value) => value.clone(),
            Value::Null => String::new(),
            value => value.to_string(),
        }
    }
    pub fn empty(&self) -> bool {
        (self.value.is_null() || self.text().trim().is_empty()) && self.formula.is_none()
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct RawSheet {
    pub name: String,
    pub start_row: usize,
    pub start_column: usize,
    pub rows: Vec<Vec<RawCell>>,
}
#[derive(Debug, Clone)]
pub struct WorkbookData {
    pub source_name: String,
    pub source_hash: String,
    pub date_1904: bool,
    pub sheets: Vec<RawSheet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportWarning {
    pub code: String,
    pub sheet: Option<String>,
    pub row: Option<usize>,
    pub column: Option<String>,
    pub message: String,
}
pub fn warning(
    code: &str,
    sheet: Option<&str>,
    row: Option<usize>,
    column: Option<&str>,
    message: &str,
) -> ImportWarning {
    ImportWarning {
        code: code.into(),
        sheet: sheet.map(str::to_owned),
        row,
        column: column.map(str::to_owned),
        message: message.into(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DateOrder {
    ReviewAmbiguous,
    DayMonthYear,
    MonthDayYear,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum PersonMapping {
    New { name: String },
    Existing { id: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PeopleMapping {
    pub carly: PersonMapping,
    pub mom: Option<PersonMapping>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HeaderOverride {
    pub sheet: String,
    pub source_header: String,
    pub destination_header: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum BookTarget {
    New,
    Existing {
        #[serde(rename = "bookId")]
        book_id: String,
    },
    Catalogue {
        #[serde(rename = "rowKey")]
        row_key: String,
    },
    Unresolved,
    Skip,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RowDecision {
    pub row_key: String,
    pub target: BookTarget,
    pub accept_confirmed_title: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportFieldMapping {
    pub source_key: String,
    pub existing_field_id: Option<String>,
    pub new_label: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportReviewDraft {
    pub selection_token: String,
    pub source_origin_id: Option<String>,
    pub people: PeopleMapping,
    pub headers: Vec<HeaderOverride>,
    pub decisions: Vec<RowDecision>,
    pub fields: Vec<ImportFieldMapping>,
    pub date_order: DateOrder,
    pub acknowledge_warnings: bool,
    pub acknowledge_missing_sheets: bool,
}
impl ImportReviewDraft {
    pub fn initial(token: &str) -> Self {
        Self {
            selection_token: token.into(),
            source_origin_id: None,
            people: PeopleMapping {
                carly: PersonMapping::New {
                    name: "Carly".into(),
                },
                mom: Some(PersonMapping::New { name: "Mom".into() }),
            },
            headers: Vec::new(),
            decisions: Vec::new(),
            fields: Vec::new(),
            date_order: DateOrder::ReviewAmbiguous,
            acknowledge_warnings: false,
            acknowledge_missing_sheets: false,
        }
    }
    pub fn validate_size(&self) -> Result<(), AppError> {
        if self.headers.len() > 256 || self.decisions.len() > 50000 || self.fields.len() > 30 {
            return Err(AppError::validation(
                "Import review contains too many mappings.",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDate {
    pub value: Option<String>,
    pub precision: String,
    pub raw: Option<RawCell>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRow {
    pub key: String,
    pub sheet: String,
    pub row_number: usize,
    pub fingerprint: String,
    pub occurrence: usize,
    pub raw: BTreeMap<String, RawCell>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueRow {
    pub source: SourceRow,
    pub book: BookDraft,
    pub reading_status: Option<ReadingStatus>,
    pub carly_rating: Option<Rating>,
    pub mom_rating: Option<Rating>,
    pub favourite: Option<bool>,
    pub renewed_to_death: Option<bool>,
    pub format: Option<String>,
    pub buy_status: Option<String>,
    pub reading_effort: Option<String>,
    pub age_range: Option<String>,
    pub borrow_count: Option<i64>,
    pub first_source_date: SourceDate,
    pub latest_source_date: SourceDate,
    pub date_type: Option<String>,
    pub original_title: Option<String>,
    pub check_title: Option<bool>,
    pub candidates: Vec<Book>,
    pub target: BookTarget,
    pub warnings: Vec<ImportWarning>,
    pub errors: Vec<String>,
    pub fill_fields: Vec<String>,
    pub preserve_fields: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BorrowRow {
    pub source: SourceRow,
    pub title: String,
    pub author: Option<String>,
    pub format: Option<String>,
    pub publication_year: Option<i64>,
    pub source_date: SourceDate,
    pub date_type: Option<String>,
    pub source_pdf: Option<String>,
    pub catalogue_candidates: Vec<String>,
    pub candidates: Vec<Book>,
    pub target: BookTarget,
    pub warnings: Vec<ImportWarning>,
    pub errors: Vec<String>,
    pub existing_loan_id: Option<String>,
    pub existing_book_id: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleReviewRow {
    pub source: SourceRow,
    pub identity_key: String,
    pub working_title: String,
    pub author: Option<String>,
    pub raw_pdf_title: Option<String>,
    pub reason: Option<String>,
    pub confirmed_title: Option<String>,
    pub catalogue_candidates: Vec<String>,
    pub candidates: Vec<Book>,
    pub target: BookTarget,
    pub accept_confirmed_title: bool,
    pub existing_task_id: Option<String>,
    pub existing_state: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetPreview {
    pub name: String,
    pub canonical_name: Option<String>,
    pub row_count: usize,
    pub header_row: Option<usize>,
    pub headers: Vec<String>,
    pub mapping: Vec<HeaderOverride>,
    pub ignored: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedField {
    pub source_key: String,
    pub label: String,
    pub scope: String,
    pub kind: String,
    pub choices: Vec<String>,
    pub existing_field_id: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCounts {
    pub catalogue: usize,
    pub historical_loans: usize,
    pub title_reviews: usize,
    pub owned_yes: usize,
    pub currently_borrowed: usize,
    pub favourites_yes: usize,
    pub mom_memberships: usize,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub draft: ImportReviewDraft,
    pub source_name: String,
    pub source_hash: String,
    pub review_hash: String,
    pub date_1904: bool,
    pub sheets: Vec<SheetPreview>,
    pub catalogue: Vec<CatalogueRow>,
    pub history: Vec<BorrowRow>,
    pub title_reviews: Vec<TitleReviewRow>,
    pub fields: Vec<ProposedField>,
    pub counts: ImportCounts,
    pub warnings: Vec<ImportWarning>,
    pub blocking_errors: Vec<String>,
    pub missing_sheets: Vec<String>,
    pub requires_title_decisions: usize,
    pub preserved_manual_values: usize,
    pub sources: Vec<ImportSource>,
    pub already_imported_batch_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ImportSource {
    pub id: String,
    pub label: String,
    pub last_filename: String,
    pub carly_person_id: String,
    pub mom_person_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub batch_id: String,
    pub origin_id: String,
    pub already_imported: bool,
    pub books_added: usize,
    pub books_updated: usize,
    pub catalogue_rows_skipped: usize,
    pub people_created: usize,
    pub memberships_created: usize,
    pub historical_loans_added: usize,
    pub historical_loans_reused: usize,
    pub unlinked_loans: usize,
    pub fields_created: usize,
    pub custom_values_written: usize,
    pub title_tasks_added: usize,
    pub unresolved_title_tasks: usize,
    pub source_rows_saved: usize,
    pub warning_count: usize,
    pub backup_name: Option<String>,
    pub carly_person_id: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ImportBatchInfo {
    pub id: String,
    pub origin_id: String,
    pub source_filename: String,
    pub source_hash: String,
    pub mapping_version: i64,
    pub imported_at: String,
    pub summary_json: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ImportedRowInfo {
    pub id: String,
    pub sheet_name: String,
    pub sheet_row_number: i64,
    pub raw_json: String,
    pub warning_json: String,
    pub book_id: Option<String>,
    pub loan_id: Option<String>,
    pub verification_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TitleTask {
    pub id: String,
    pub book_id: Option<String>,
    pub current_title: Option<String>,
    pub working_title: String,
    pub raw_pdf_title: Option<String>,
    pub reason: Option<String>,
    pub proposed_confirmed_title: Option<String>,
    pub state: String,
    pub source_row_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TitleResolution {
    pub book_id: Option<String>,
    pub confirmed_title: Option<String>,
    pub expected_current_title: Option<String>,
    pub expected_state: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalLoan {
    pub id: String,
    pub book_id: Option<String>,
    pub current_title: Option<String>,
    pub person_id: String,
    pub person_name: String,
    pub source_title: String,
    pub source_author: Option<String>,
    pub source_format: Option<String>,
    pub publication_year: Option<i64>,
    pub source_date_json: String,
    pub date_interpretation: String,
    pub date_precision: String,
    pub source_pdf: Option<String>,
    pub imported_row_id: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BorrowSummary {
    pub book_id: String,
    pub imported_count_baseline: Option<i64>,
    pub first_source_date_json: String,
    pub latest_source_date_json: String,
    pub source_date_type: Option<String>,
    pub linked_history_count: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CatalogueIdentity {
    pub source_row_key: String,
    pub book_id: Option<String>,
    pub skipped: bool,
    pub original_title: Option<String>,
    pub source_title: String,
    pub source_author: Option<String>,
    pub source_row_number: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LoanIdentity {
    pub id: String,
    pub source_row_key: String,
    pub book_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TaskIdentity {
    pub id: String,
    pub source_row_key: String,
    pub book_id: Option<String>,
    pub state: String,
    pub proposed_confirmed_title: Option<String>,
}

pub fn fill_book(mut current: BookDraft, incoming: &BookDraft) -> (BookDraft, bool) {
    fn fill<T: Clone>(current: &mut Option<T>, incoming: &Option<T>) -> bool {
        if current.is_none() && incoming.is_some() {
            *current = incoming.clone();
            true
        } else {
            false
        }
    }
    let mut changed = false;
    macro_rules! fields { ($($field:ident),+) => { $(changed |= fill(&mut current.$field, &incoming.$field);)+ }; }
    fields!(
        subtitle,
        authors,
        isbn13,
        publisher,
        publication_year,
        language,
        series,
        volume,
        description,
        notes,
        owned
    );
    (current, changed)
}

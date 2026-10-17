use crate::{
    domain::{
        books::{self, BookDraft, Rating, ReadingStatus},
        custom_fields::{FieldDefinition, Kind, Scope},
        import::*,
        AppError,
    },
    services::{books as book_service, custom_fields, household, xlsx_reader},
};
use calamine::{ExcelDateTime, ExcelDateTimeType};
use sqlx::SqlitePool;
use std::collections::{BTreeMap, HashMap, HashSet};

struct MappedRow {
    source: SourceRow,
    values: BTreeMap<String, RawCell>,
}
impl MappedRow {
    fn cell(&self, header: &str) -> Option<&RawCell> {
        self.values.get(header).filter(|cell| !cell.empty())
    }
    fn text(&self, header: &str) -> Option<String> {
        self.cell(header)
            .map(RawCell::text)
            .filter(|value| !value.trim().is_empty())
    }
    fn author(&self) -> Option<String> {
        self.text("Author")
            .map(|value| value.trim().to_string())
            .filter(|value| value != "??")
    }
    fn warn(&self, warnings: &mut Vec<ImportWarning>, code: &str, column: &str, message: &str) {
        warnings.push(warning(
            code,
            Some(&self.source.sheet),
            Some(self.source.row_number),
            Some(column),
            message,
        ));
    }
}
fn normalize(value: &str) -> String {
    books::normalized(value)
}
fn normalize_header(value: &str) -> String {
    normalize(value).replace('\u{a0}', " ")
}
fn canonical_sheet(value: &str) -> Option<&'static str> {
    SHEETS
        .into_iter()
        .find(|sheet| normalize(sheet) == normalize(value))
}

fn map_sheet(
    sheet: &RawSheet,
    canonical: &str,
    draft: &ImportReviewDraft,
    warnings: &mut Vec<ImportWarning>,
    errors: &mut Vec<String>,
) -> (SheetPreview, Vec<MappedRow>) {
    let expected = expected_headers(canonical);
    let ignored = expected.is_empty();
    let required = if canonical == TITLES {
        "Working Title"
    } else {
        "Title"
    };
    let first_nonempty = sheet
        .rows
        .iter()
        .position(|row| row.iter().any(|cell| !cell.empty()));
    let mut header_index = first_nonempty;
    let mut best_score = 0;
    for (index, row) in sheet.rows.iter().take(20).enumerate() {
        let score = row
            .iter()
            .filter(|cell| {
                expected
                    .iter()
                    .any(|header| normalize_header(header) == normalize_header(&cell.text()))
                    || draft.headers.iter().any(|mapping| {
                        mapping.sheet == canonical
                            && mapping.source_header == cell.text().trim()
                            && mapping.destination_header.is_some()
                    })
            })
            .count();
        if score > best_score && (score >= 2 || Some(index) == first_nonempty) {
            header_index = Some(index);
            best_score = score;
        }
    }
    if ignored {
        return (
            SheetPreview {
                name: sheet.name.clone(),
                canonical_name: Some(canonical.into()),
                row_count: sheet
                    .rows
                    .iter()
                    .filter(|row| row.iter().any(|cell| !cell.empty()))
                    .count(),
                header_row: None,
                headers: Vec::new(),
                mapping: Vec::new(),
                ignored: true,
            },
            Vec::new(),
        );
    }
    let Some(header_index) = header_index else {
        errors.push(format!("{canonical} has no header row."));
        return (
            SheetPreview {
                name: sheet.name.clone(),
                canonical_name: Some(canonical.into()),
                row_count: 0,
                header_row: None,
                headers: Vec::new(),
                mapping: Vec::new(),
                ignored: false,
            },
            Vec::new(),
        );
    };
    if sheet.rows[..header_index]
        .iter()
        .any(|row| row.iter().any(|cell| !cell.empty()))
    {
        warnings.push(warning("preamble_rows", Some(canonical), None, None, "Nonempty rows before the detected headers are reference text and are not imported as records. Keep the original workbook for context."));
    }
    let headers: Vec<String> = sheet.rows[header_index]
        .iter()
        .enumerate()
        .map(|(index, cell)| {
            if cell.text().trim().is_empty() {
                format!("(unnamed column {})", sheet.start_column + index + 1)
            } else {
                cell.text().trim().to_string()
            }
        })
        .collect();
    let mut seen_headers = HashSet::new();
    let mut mapped_destinations = HashSet::new();
    let mapping: Vec<HeaderOverride> = headers
        .iter()
        .map(|header| {
            if !seen_headers.insert(normalize_header(header)) {
                errors.push(format!("{canonical} has ambiguous duplicate headers."));
            }
            let override_mapping = draft
                .headers
                .iter()
                .find(|mapping| mapping.sheet == canonical && mapping.source_header == *header);
            let destination = override_mapping
                .map(|mapping| mapping.destination_header.clone())
                .unwrap_or_else(|| {
                    expected
                        .iter()
                        .find(|expected| normalize_header(expected) == normalize_header(header))
                        .map(|header| (*header).to_string())
                });
            if let Some(destination) = &destination {
                if !expected.contains(&destination.as_str())
                    || !mapped_destinations.insert(destination.clone())
                {
                    errors.push(format!(
                        "{canonical} has an invalid or duplicate destination mapping."
                    ));
                }
            }
            HeaderOverride {
                sheet: canonical.into(),
                source_header: header.clone(),
                destination_header: destination,
            }
        })
        .collect();
    if !mapped_destinations.contains(required) {
        errors.push(format!(
            "{canonical} requires the {required} header. Adjust the header mapping."
        ));
    }
    for header in expected {
        if !mapped_destinations.contains(*header) && *header != required {
            warnings.push(warning("missing_optional_header", Some(canonical), None, Some(header), "This column is absent. Its values remain unknown; no existing value will be erased."));
        }
    }
    let mut occurrences: HashMap<String, usize> = HashMap::new();
    let mut rows = Vec::new();
    let mut unmapped: BTreeMap<String, usize> = BTreeMap::new();
    for (index, cells) in sheet.rows.iter().enumerate().skip(header_index + 1) {
        if cells.iter().all(RawCell::empty) {
            continue;
        }
        let mut raw = BTreeMap::new();
        let mut values = BTreeMap::new();
        let mut stable = BTreeMap::new();
        for (column, cell) in cells.iter().enumerate() {
            let header = headers
                .get(column)
                .cloned()
                .unwrap_or_else(|| format!("(unnamed column {})", sheet.start_column + column + 1));
            raw.insert(header.clone(), cell.clone());
            let destination = mapping
                .get(column)
                .and_then(|mapping| mapping.destination_header.as_deref());
            let identity_header = destination
                .map(str::to_string)
                .unwrap_or_else(|| header.clone());
            if !cell.empty() {
                stable.insert(
                    normalize_header(&identity_header),
                    normalize(cell.calendar_date.as_deref().unwrap_or(&cell.text())),
                );
            }
            if let Some(destination) = destination {
                values.insert(destination.into(), cell.clone());
            } else if !cell.empty() {
                *unmapped.entry(header).or_default() += 1;
            }
        }
        let fingerprint = xlsx_reader::hash(
            serde_json::to_string(&stable)
                .unwrap_or_default()
                .as_bytes(),
        );
        let occurrence = occurrences.entry(fingerprint.clone()).or_default();
        *occurrence += 1;
        let key = xlsx_reader::hash(
            format!("{}:{fingerprint}:{}", normalize(canonical), occurrence).as_bytes(),
        );
        let source = SourceRow {
            key,
            sheet: canonical.into(),
            row_number: sheet.start_row + index + 1,
            fingerprint,
            occurrence: *occurrence,
            raw,
        };
        for (header, cell) in &source.raw {
            if cell.formula.is_some() {
                warnings.push(warning("cached_formula", Some(canonical), Some(source.row_number), Some(header), "Formula text is retained as provenance. Only the cached value is read; formulas are never executed."));
            }
        }
        rows.push(MappedRow { source, values });
    }
    for (column, count) in unmapped {
        warnings.push(warning(
            "unmapped_column",
            Some(canonical),
            None,
            Some(&column),
            &format!(
                "{count} nonempty source cells are unmapped and will be preserved in provenance."
            ),
        ));
    }
    (
        SheetPreview {
            name: sheet.name.clone(),
            canonical_name: Some(canonical.into()),
            row_count: rows.len(),
            header_row: Some(sheet.start_row + header_index + 1),
            headers,
            mapping,
            ignored: false,
        },
        rows,
    )
}

fn boolean(row: &MappedRow, header: &str, warnings: &mut Vec<ImportWarning>) -> Option<bool> {
    let cell = row.cell(header)?;
    if let Some(value) = cell.value.as_bool() {
        return Some(value);
    }
    match normalize(&cell.text()).as_str() {
        "yes" => Some(true),
        "no" => Some(false),
        _ => {
            row.warn(warnings, "unsupported_boolean", header, "Expected Yes, No or blank. The original value is retained and the mapped value remains unknown.");
            None
        }
    }
}
fn integer(
    row: &MappedRow,
    header: &str,
    year: bool,
    warnings: &mut Vec<ImportWarning>,
) -> Option<i64> {
    let cell = row.cell(header)?;
    let value = cell
        .value
        .as_i64()
        .or_else(|| {
            cell.value
                .as_f64()
                .filter(|value| {
                    value.is_finite() && value.fract() == 0.0 && value.abs() < 1_000_000_001.0
                })
                .map(|value| value as i64)
        })
        .or_else(|| cell.text().trim().parse::<i64>().ok());
    if let Some(value) = value.filter(|value| {
        if year {
            (1..=3000).contains(value)
        } else {
            (0..=1_000_000_000).contains(value)
        }
    }) {
        Some(value)
    } else {
        row.warn(warnings, "invalid_number", header, "The numeric value is invalid for this column. The original value is retained; no zero or year is invented.");
        None
    }
}
fn rating(row: &MappedRow, header: &str, warnings: &mut Vec<ImportWarning>) -> Option<Rating> {
    let value = row.text(header)?;
    match normalize(&value).as_str() {
        "loved" => Some(Rating::Loved),
        "liked" => Some(Rating::Liked),
        "meh" => Some(Rating::Meh),
        "nope" => Some(Rating::Nope),
        _ => {
            row.warn(warnings, "unsupported_rating", header, "The rating is not Loved, Liked, Meh or Nope. It is preserved for review and remains unknown.");
            None
        }
    }
}
fn status(row: &MappedRow, warnings: &mut Vec<ImportWarning>) -> Option<ReadingStatus> {
    let value = row.text("Reading Status")?;
    match normalize(&value).as_str() {
        "read" => Some(ReadingStatus::Read),
        "want to read" => Some(ReadingStatus::WantToRead),
        "on hold" => Some(ReadingStatus::OnHold),
        "currently borrowed" => Some(ReadingStatus::CurrentlyBorrowed),
        "reading" => Some(ReadingStatus::Reading),
        "did not finish" => Some(ReadingStatus::DidNotFinish),
        _ => {
            row.warn(
                warnings,
                "unsupported_status",
                "Reading Status",
                "The reading status is unsupported. The original label is retained for review.",
            );
            None
        }
    }
}
fn source_date(
    row: &MappedRow,
    header: &str,
    order: &DateOrder,
    date_1904: bool,
    warnings: &mut Vec<ImportWarning>,
) -> SourceDate {
    let Some(cell) = row.cell(header) else {
        return SourceDate {
            value: None,
            precision: "unknown".into(),
            raw: None,
        };
    };
    let mut date = cell
        .calendar_date
        .clone()
        .filter(|date| crate::domain::custom_fields::valid_date(date));
    if !date_1904
        && cell.kind == "excel_date"
        && cell
            .value
            .as_f64()
            .is_some_and(|serial| serial.floor() == 60.0)
    {
        date = None;
    }
    let mut precision = "day";
    if date.is_none() && matches!(cell.kind.as_str(), "number" | "integer" | "excel_date") {
        if let Some(serial) = cell.value.as_f64().filter(|serial| {
            serial.is_finite()
                && *serial >= if date_1904 { 0.0 } else { 1.0 }
                && *serial <= 2_958_465.0
                && (date_1904 || serial.floor() != 60.0)
        }) {
            date = ExcelDateTime::new(serial, ExcelDateTimeType::DateTime, date_1904)
                .as_datetime()
                .map(|date| date.date().to_string());
            if cell.kind != "excel_date" {
                row.warn(warnings, "inferred_excel_date", header, "A numeric value in a source-date column was interpreted as an Excel serial date. It remains a source date, not a verified checkout or reading date.");
            }
        }
    }
    if date.is_none() && matches!(cell.kind.as_str(), "text" | "iso_date") {
        let text = cell.text();
        let text = text.trim();
        if crate::domain::custom_fields::valid_date(text) {
            date = Some(text.into());
        } else if text.len() == 7
            && text.as_bytes().get(4) == Some(&b'-')
            && text[..4].parse::<u16>().is_ok_and(|year| year > 0)
            && text[5..]
                .parse::<u8>()
                .is_ok_and(|month| (1..=12).contains(&month))
        {
            date = Some(text.into());
            precision = "month";
        } else if text.len() == 4 && text.parse::<u16>().is_ok_and(|year| year > 0) {
            date = Some(text.into());
            precision = "year";
        } else {
            let parts: Vec<&str> = text.split('/').collect();
            if parts.len() == 3 {
                if let (Ok(first), Ok(second), Ok(year)) = (
                    parts[0].parse::<u8>(),
                    parts[1].parse::<u8>(),
                    parts[2].parse::<i32>(),
                ) {
                    let ambiguous = first <= 12 && second <= 12 && first != second;
                    let day_first = match order {
                        DateOrder::DayMonthYear => Some(true),
                        DateOrder::MonthDayYear => Some(false),
                        DateOrder::ReviewAmbiguous => {
                            if ambiguous {
                                None
                            } else {
                                Some(first > 12 || first == second)
                            }
                        }
                    };
                    if let Some(day_first) = day_first {
                        let (day, month) = if day_first {
                            (first, second)
                        } else {
                            (second, first)
                        };
                        let candidate = format!("{year:04}-{month:02}-{day:02}");
                        if crate::domain::custom_fields::valid_date(&candidate) {
                            date = Some(candidate);
                        }
                    } else {
                        row.warn(warnings, "ambiguous_date", header, "This slash date is ambiguous. Choose an explicit date order to interpret it, or keep the original unknown date for review.");
                    }
                }
            }
        }
    }
    if date.is_none() {
        precision = "unknown";
        row.warn(warnings, "invalid_source_date", header, "The source date could not be interpreted safely. Its original type and value are retained without inventing a date.");
    }
    SourceDate {
        value: date,
        precision: precision.into(),
        raw: Some(cell.clone()),
    }
}

fn candidates(title: &str, author: Option<&str>, books: &[books::Book]) -> Vec<books::Book> {
    books
        .iter()
        .filter(|book| {
            normalize(&book.data.title) == normalize(title)
                && author.is_none_or(|author| {
                    normalize(book.data.authors.as_deref().unwrap_or("")) == normalize(author)
                })
        })
        .cloned()
        .collect()
}
fn target(draft: &ImportReviewDraft, key: &str, default: BookTarget) -> BookTarget {
    draft
        .decisions
        .iter()
        .find(|decision| decision.row_key == key)
        .map(|decision| decision.target.clone())
        .unwrap_or(default)
}
fn field_proposals(
    catalogue: &[CatalogueRow],
    definitions: &[FieldDefinition],
    draft: &ImportReviewDraft,
    errors: &mut Vec<String>,
    previous_fields: &BTreeMap<String, String>,
) -> Vec<ProposedField> {
    let specs = [
        ("format", "Format", Scope::Book, Kind::Select),
        ("buy_status", "Buy Status", Scope::Book, Kind::Select),
        ("age_range", "Age Range", Scope::Book, Kind::Text),
        ("favourite", "Favourite?", Scope::PersonBook, Kind::Boolean),
        (
            "reading_effort",
            "Reading Effort",
            Scope::PersonBook,
            Kind::Text,
        ),
        (
            "renewed_to_death",
            "Renewed to Death?",
            Scope::PersonBook,
            Kind::Boolean,
        ),
    ];
    specs.into_iter().map(|(key, label, scope, kind)| {
        let mapping = draft.fields.iter().find(|mapping| mapping.source_key == key);
        let matching = definitions.iter().find(|definition| definition.draft.active && definition.draft.scope == scope && definition.draft.kind == kind && normalize(&definition.draft.label) == normalize(label));
        let existing_field_id = mapping.map(|mapping| mapping.existing_field_id.clone()).unwrap_or_else(|| previous_fields.get(key).cloned().or_else(|| matching.map(|definition| definition.id.clone())));
        if let Some(id) = &existing_field_id { if !definitions.iter().any(|definition| definition.id == *id && definition.draft.active && definition.draft.scope == scope && definition.draft.kind == kind) { errors.push(format!("Custom field mapping for {label} has an incompatible field ID, scope or type.")); } }
        let label = mapping.map(|mapping| mapping.new_label.clone()).unwrap_or_else(|| label.into());
        if existing_field_id.is_none() && (crate::domain::person_name(&label).is_err() || definitions.iter().any(|definition| definition.draft.active && definition.draft.scope == scope && normalize(&definition.draft.label) == normalize(&label))) { errors.push(format!("Choose a distinct valid label or compatible existing field for {key}.")); }
        let mut choices: Vec<String> = catalogue.iter().filter_map(|row| match key { "format" => row.format.clone(), "buy_status" => row.buy_status.clone(), _ => None }).collect(); choices.sort(); choices.dedup();
        ProposedField { source_key: key.into(), label, scope: match scope { Scope::Book => "book", Scope::PersonBook => "person_book" }.into(), kind: match kind { Kind::Select => "select", Kind::Text => "text", _ => "boolean" }.into(), choices, existing_field_id }
    }).collect()
}

pub async fn preview(
    pool: &SqlitePool,
    workbook: &WorkbookData,
    draft: ImportReviewDraft,
) -> Result<ImportPreview, AppError> {
    draft.validate_size()?;
    let existing_details = book_service::list(pool, books::BookFilter::default()).await?;
    let existing: Vec<books::Book> = existing_details
        .iter()
        .map(|row| row.book.clone())
        .collect();
    let people = household::list(pool).await?;
    let definitions = custom_fields::list(pool).await?;
    let sources = crate::persistence::import::sources(pool).await?;
    let already_imported_batch_id =
        crate::persistence::import::batch_by_hash(pool, &workbook.source_hash)
            .await?
            .map(|batch| batch.id);
    let previous_fields = match &draft.source_origin_id {
        Some(origin) => crate::persistence::import::field_mappings(pool, origin).await?,
        None => BTreeMap::new(),
    };
    let identities = match &draft.source_origin_id {
        Some(origin) => crate::persistence::import::catalogue_identities(pool, origin).await?,
        None => Vec::new(),
    };
    let prior_loans = match &draft.source_origin_id {
        Some(origin) => crate::persistence::import::loan_identities(pool, origin).await?,
        None => Vec::new(),
    };
    let prior_tasks = match &draft.source_origin_id {
        Some(origin) => crate::persistence::import::task_identities(pool, origin).await?,
        None => Vec::new(),
    };
    let review_payload = serde_json::to_vec(&(
        &workbook.source_hash,
        &draft,
        &existing_details,
        &people.people,
        &definitions,
        &prior_loans,
        &prior_tasks,
        &previous_fields,
        &identities,
        &sources,
    ))
    .map_err(|_| AppError::validation("The import preview could not be prepared."))?;
    let review_hash = xlsx_reader::hash(&review_payload);
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    if draft
        .source_origin_id
        .as_ref()
        .is_some_and(|id| !sources.iter().any(|source| &source.id == id))
    {
        errors.push("The selected previous import source does not exist.".into());
    }
    for mapping in [&draft.people.carly]
        .into_iter()
        .chain(draft.people.mom.iter())
    {
        match mapping {
            PersonMapping::New { name } => {
                if let Err(error) = crate::domain::person_name(name) {
                    errors.push(error.message);
                }
            }
            PersonMapping::Existing { id } => {
                if !people
                    .people
                    .iter()
                    .any(|person| person.id == *id && person.archived_at.is_none())
                {
                    errors.push("Choose an active existing person for the source columns.".into());
                }
            }
        }
    }
    if matches!((&draft.people.carly, &draft.people.mom), (PersonMapping::Existing { id: first }, Some(PersonMapping::Existing { id: second })) if first == second)
    {
        errors.push("Carly and Mom must map to separate local people.".into());
    }
    let mut mapped: HashMap<String, Vec<MappedRow>> = HashMap::new();
    let mut sheets = Vec::new();
    let mut seen = HashSet::new();
    for sheet in &workbook.sheets {
        if let Some(canonical) = canonical_sheet(&sheet.name) {
            if !seen.insert(canonical) {
                errors.push(format!("Multiple sheets normalize to {canonical}. Rename or remove the ambiguous sheet before import."));
                continue;
            }
            let (overview, rows) = map_sheet(sheet, canonical, &draft, &mut warnings, &mut errors);
            sheets.push(overview);
            mapped.insert(canonical.into(), rows);
        } else {
            warnings.push(warning(
                "unknown_sheet",
                Some(&sheet.name),
                None,
                None,
                "This unrecognized sheet is not imported as books. The workbook remains unchanged.",
            ));
            sheets.push(SheetPreview {
                name: sheet.name.clone(),
                canonical_name: None,
                row_count: sheet
                    .rows
                    .iter()
                    .filter(|row| row.iter().any(|cell| !cell.empty()))
                    .count(),
                header_row: None,
                headers: Vec::new(),
                mapping: Vec::new(),
                ignored: true,
            });
        }
    }
    let missing_sheets: Vec<String> = SHEETS
        .into_iter()
        .filter(|sheet| !seen.contains(sheet))
        .map(str::to_owned)
        .collect();
    if !seen.contains(CATALOGUE) {
        errors.push("The required Book Catalogue sheet is missing.".into());
    }
    for sheet in &missing_sheets {
        warnings.push(warning("missing_sheet", Some(sheet), None, None, if sheet == HISTORY || sheet == TITLES { "This source-data sheet is missing. Explicitly acknowledge the incomplete workbook before importing the remaining records." } else { "This optional reference sheet is missing. It is never used to create books or reading events." }));
    }
    let mut catalogue = Vec::new();
    for row in mapped.remove(CATALOGUE).unwrap_or_default() {
        let mut row_warnings = Vec::new();
        let mut row_errors = Vec::new();
        let reading_status = status(&row, &mut row_warnings);
        let carly_rating = rating(&row, "Carly Rating", &mut row_warnings);
        let mom_rating = rating(&row, "Mom Rating", &mut row_warnings);
        let favourite = boolean(&row, "Favourite?", &mut row_warnings);
        let renewed_to_death = boolean(&row, "Renewed to Death?", &mut row_warnings);
        let owned = boolean(&row, "Owned?", &mut row_warnings);
        let check_title = boolean(&row, "Check Title?", &mut row_warnings);
        let publication_year = integer(&row, "Published", true, &mut row_warnings);
        let borrow_count = integer(&row, "Borrow Count", false, &mut row_warnings);
        let first_source_date = source_date(
            &row,
            "First Library Date",
            &draft.date_order,
            workbook.date_1904,
            &mut row_warnings,
        );
        let latest_source_date = source_date(
            &row,
            "Latest Library Date",
            &draft.date_order,
            workbook.date_1904,
            &mut row_warnings,
        );
        let book = BookDraft {
            title: row.text("Title").unwrap_or_default().trim().into(),
            authors: row.author(),
            publication_year,
            notes: row.text("Notes"),
            owned,
            ..Default::default()
        };
        if let Err(error) = book.clone().validate() {
            row_errors.push(error.message);
        }
        let mut existing_candidates = candidates(&book.title, book.authors.as_deref(), &existing);
        let (prior_target, prior_candidates) = previous_catalogue_target(
            &identities,
            &row.source,
            &book,
            row.text("Source / Original Title").as_deref(),
        );
        for id in prior_candidates {
            if !existing_candidates
                .iter()
                .any(|candidate| candidate.id == id)
            {
                if let Some(book) = existing.iter().find(|book| book.id == id) {
                    existing_candidates.push(book.clone());
                }
            }
        }
        let chosen = target(
            &draft,
            &row.source.key,
            prior_target.unwrap_or({
                if existing_candidates.is_empty() {
                    BookTarget::New
                } else {
                    BookTarget::Unresolved
                }
            }),
        );
        let mut fill_fields = Vec::new();
        let mut preserve_fields = Vec::new();
        if let BookTarget::Existing { book_id } = &chosen {
            if let Some(current) = existing.iter().find(|book| &book.id == book_id) {
                for (label, incoming, populated) in [
                    (
                        "Title",
                        !book.title.is_empty(),
                        !current.data.title.is_empty(),
                    ),
                    (
                        "Author",
                        book.authors.is_some(),
                        current.data.authors.is_some(),
                    ),
                    (
                        "Published",
                        book.publication_year.is_some(),
                        current.data.publication_year.is_some(),
                    ),
                    ("Owned?", book.owned.is_some(), current.data.owned.is_some()),
                    ("Notes", book.notes.is_some(), current.data.notes.is_some()),
                ] {
                    if incoming {
                        if populated {
                            preserve_fields.push(label.into());
                        } else {
                            fill_fields.push(label.into());
                        }
                    }
                }
                if let Some(detail) = existing_details
                    .iter()
                    .find(|detail| &detail.book.id == book_id)
                {
                    let carly = match &draft.people.carly {
                        PersonMapping::Existing { id } => detail
                            .memberships
                            .iter()
                            .find(|membership| &membership.person_id == id),
                        _ => None,
                    };
                    let mom = match &draft.people.mom {
                        Some(PersonMapping::Existing { id }) => detail
                            .memberships
                            .iter()
                            .find(|membership| &membership.person_id == id),
                        _ => None,
                    };
                    for (label, incoming, populated) in [
                        (
                            "Carly reading status",
                            reading_status.is_some(),
                            carly.is_some_and(|membership| membership.reading_status.is_some()),
                        ),
                        (
                            "Carly rating",
                            carly_rating.is_some(),
                            carly.is_some_and(|membership| membership.rating.is_some()),
                        ),
                        (
                            "Mom rating",
                            mom_rating.is_some(),
                            mom.is_some_and(|membership| membership.rating.is_some()),
                        ),
                    ] {
                        if incoming {
                            if populated {
                                preserve_fields.push(label.into());
                            } else {
                                fill_fields.push(label.into());
                            }
                        }
                    }
                }
            } else {
                row_errors.push("The selected existing book does not exist.".into());
            }
        }
        if mom_rating.is_some() && draft.people.mom.is_none() {
            errors.push(
                "Mom has source ratings. Select a separate local person to preserve them.".into(),
            );
        }
        let entry = CatalogueRow {
            source: row.source.clone(),
            book,
            reading_status,
            carly_rating,
            mom_rating,
            favourite,
            renewed_to_death,
            format: row.text("Format"),
            buy_status: row.text("Buy Status"),
            reading_effort: row.text("Reading Effort"),
            age_range: row.text("Age Range"),
            borrow_count,
            first_source_date,
            latest_source_date,
            date_type: row.text("Date Type"),
            original_title: row.text("Source / Original Title"),
            check_title,
            candidates: existing_candidates,
            target: chosen,
            warnings: row_warnings,
            errors: row_errors,
            fill_fields,
            preserve_fields,
        };
        warnings.extend(entry.warnings.clone());
        catalogue.push(entry);
    }
    let mut duplicate_titles = HashSet::new();
    for row in &catalogue {
        if !duplicate_titles.insert((
            normalize(&row.book.title),
            normalize(row.book.authors.as_deref().unwrap_or("")),
        )) {
            warnings.push(warning("duplicate_catalogue_title", Some(CATALOGUE), Some(row.source.row_number), Some("Title"), "Another catalogue row has the same normalized title and author. They remain separate unless explicitly resolved to the same book."));
        }
    }
    let mut history = Vec::new();
    for row in mapped.remove(HISTORY).unwrap_or_default() {
        let mut row_warnings = Vec::new();
        let title = row.text("Title").unwrap_or_default().trim().to_string();
        let author = row.author();
        let source_date = source_date(
            &row,
            "Approximate Borrow Date",
            &draft.date_order,
            workbook.date_1904,
            &mut row_warnings,
        );
        let publication_year = integer(&row, "Publication Year", true, &mut row_warnings);
        let catalogue_candidates: Vec<String> = catalogue
            .iter()
            .filter(|candidate| {
                normalize(&candidate.book.title) == normalize(&title)
                    && normalize(candidate.book.authors.as_deref().unwrap_or(""))
                        == normalize(author.as_deref().unwrap_or(""))
            })
            .map(|row| row.source.key.clone())
            .collect();
        let candidates = candidates(&title, author.as_deref(), &existing);
        let known_loan = prior_loans
            .iter()
            .find(|loan| loan.source_row_key == row.source.key);
        let default = if let Some(loan) = known_loan {
            loan.book_id
                .as_ref()
                .map(|id| BookTarget::Existing {
                    book_id: id.clone(),
                })
                .unwrap_or(BookTarget::Unresolved)
        } else if catalogue_candidates.len() == 1 {
            BookTarget::Catalogue {
                row_key: catalogue_candidates[0].clone(),
            }
        } else {
            BookTarget::Unresolved
        };
        let chosen = target(&draft, &row.source.key, default);
        if let Some(loan) = known_loan {
            let expected = loan
                .book_id
                .as_ref()
                .map(|id| BookTarget::Existing {
                    book_id: id.clone(),
                })
                .unwrap_or(BookTarget::Unresolved);
            if crate::persistence::import::encode(&chosen)?
                != crate::persistence::import::encode(&expected)?
            {
                errors.push(format!("History row {} already has a saved historical record. Keep its existing link here; changes can be reviewed in Import History after import.", row.source.row_number));
            }
        }
        if matches!(chosen, BookTarget::Unresolved) {
            row.warn(&mut row_warnings, "unlinked_borrow", "Title", "No unique catalogue match is established. Keep this historical record unlinked or explicitly choose a book; no fuzzy merge is performed.");
        }
        if title.is_empty() {
            row.warn(&mut row_warnings, "missing_history_title", "Title", "The historical row has no title. The source row is retained as an unlinked historical record.");
        }
        warnings.extend(row_warnings.clone());
        history.push(BorrowRow {
            source: row.source.clone(),
            title,
            author,
            format: row.text("Format"),
            publication_year,
            source_date,
            date_type: row.text("Actual Date Type"),
            source_pdf: row.text("Source PDF"),
            catalogue_candidates,
            candidates,
            target: chosen,
            warnings: row_warnings,
            errors: Vec::new(),
            existing_loan_id: known_loan.map(|loan| loan.id.clone()),
            existing_book_id: known_loan.and_then(|loan| loan.book_id.clone()),
        });
    }
    let mut title_reviews = Vec::new();
    let mut title_occurrences: HashMap<String, usize> = HashMap::new();
    for row in mapped.remove(TITLES).unwrap_or_default() {
        let working_title = row.text("Working Title").unwrap_or_default();
        let author = row.author();
        let anchor = xlsx_reader::hash(
            serde_json::to_string(&(
                normalize(&working_title),
                normalize(author.as_deref().unwrap_or("")),
                normalize(row.text("Exactly as PDF shows").as_deref().unwrap_or("")),
            ))
            .map_err(|_| AppError::validation("Invalid title source evidence."))?
            .as_bytes(),
        );
        let occurrence = title_occurrences.entry(anchor.clone()).or_default();
        *occurrence += 1;
        let identity_key = xlsx_reader::hash(format!("title:{anchor}:{occurrence}").as_bytes());
        let known_task = prior_tasks
            .iter()
            .find(|task| task.source_row_key == identity_key);
        let catalogue_candidates: Vec<String> = catalogue
            .iter()
            .filter(|candidate| {
                normalize(&candidate.book.title) == normalize(&working_title)
                    && normalize(candidate.book.authors.as_deref().unwrap_or(""))
                        == normalize(author.as_deref().unwrap_or(""))
            })
            .map(|row| row.source.key.clone())
            .collect();
        let candidates = candidates(&working_title, author.as_deref(), &existing);
        let default = if let Some(task) = known_task {
            task.book_id
                .as_ref()
                .map(|id| BookTarget::Existing {
                    book_id: id.clone(),
                })
                .unwrap_or(BookTarget::Unresolved)
        } else if catalogue_candidates.len() == 1 {
            BookTarget::Catalogue {
                row_key: catalogue_candidates[0].clone(),
            }
        } else {
            BookTarget::Unresolved
        };
        let chosen = target(&draft, &row.source.key, default);
        if let Some(task) = known_task {
            let expected = task
                .book_id
                .as_ref()
                .map(|id| BookTarget::Existing {
                    book_id: id.clone(),
                })
                .unwrap_or(BookTarget::Unresolved);
            if crate::persistence::import::encode(&chosen)?
                != crate::persistence::import::encode(&expected)?
            {
                errors.push(format!("Title row {} already has a saved verification task. Keep its existing link here; review link changes in Title Verification.", row.source.row_number));
            }
        }
        let accept_confirmed_title = draft
            .decisions
            .iter()
            .find(|decision| decision.row_key == row.source.key)
            .is_some_and(|decision| decision.accept_confirmed_title);
        if accept_confirmed_title {
            if known_task.is_some_and(|task| task.state != "open") {
                errors.push(format!("Title row {} is already resolved. Review further corrections in Title Verification.", row.source.row_number));
            }
            match row.text("Confirmed Full Title (edit here)") {
                Some(title)
                    if (BookDraft {
                        title: title.clone(),
                        ..Default::default()
                    })
                    .validate()
                    .is_ok() => {}
                _ => errors.push(format!(
                    "Title row {} needs a valid confirmed title before applying a correction.",
                    row.source.row_number
                )),
            }
        }
        title_reviews.push(TitleReviewRow {
            source: row.source.clone(),
            identity_key,
            working_title,
            author,
            raw_pdf_title: row.text("Exactly as PDF shows"),
            reason: row.text("Reason"),
            confirmed_title: row.text("Confirmed Full Title (edit here)"),
            catalogue_candidates,
            candidates,
            target: chosen,
            accept_confirmed_title,
            existing_task_id: known_task.map(|task| task.id.clone()),
            existing_state: known_task.map(|task| task.state.clone()),
        });
    }
    for row in &catalogue {
        let reconstructed = history
            .iter()
            .filter(|history| match (&row.target, &history.target) {
                (BookTarget::Skip, _) => false,
                (_, BookTarget::Catalogue { row_key }) => row_key == &row.source.key,
                (BookTarget::Existing { book_id }, BookTarget::Existing { book_id: other }) => {
                    book_id == other
                }
                _ => false,
            })
            .count() as i64;
        if row
            .borrow_count
            .is_some_and(|baseline| baseline != reconstructed)
        {
            warnings.push(warning("borrow_count_mismatch", Some(CATALOGUE), Some(row.source.row_number), Some("Borrow Count"), &format!("The source baseline is {}, while {reconstructed} history rows currently match. The baseline is preserved; missing transactions are not fabricated.", row.borrow_count.unwrap_or_default())));
        }
    }
    let fields = field_proposals(
        &catalogue,
        &definitions,
        &draft,
        &mut errors,
        &previous_fields,
    );
    for field in &fields {
        for choice in &field.choices {
            if crate::domain::person_name(choice).is_err() {
                warnings.push(warning("unsupported_choice", Some(CATALOGUE), None, Some(&field.label), "A source choice exceeds the supported label limits. It is retained in provenance without a custom value."));
            }
        }
        if let Some(definition) = definitions
            .iter()
            .find(|definition| Some(&definition.id) == field.existing_field_id.as_ref())
        {
            if definition.draft.options.iter().any(|option| !option.active) {
                warnings.push(warning("retired_choices", Some(CATALOGUE), None, Some(&field.label), "This mapped field has retired choices. Source values mapped to retired IDs stay in provenance; those choices are not reactivated."));
            }
            for row in &catalogue {
                let text = match field.source_key.as_str() {
                    "age_range" => &row.age_range,
                    "reading_effort" => &row.reading_effort,
                    _ => continue,
                };
                if text.as_ref().is_some_and(|text| {
                    definition
                        .draft
                        .validate_value(&serde_json::Value::String(text.clone()), false)
                        .is_err()
                }) {
                    warnings.push(warning("unsupported_custom_value", Some(CATALOGUE), Some(row.source.row_number), Some(&field.label), "The source value does not fit this field's validation rules. It is retained in provenance without replacing the existing value."));
                }
            }
        }
    }
    let requires_title_decisions = catalogue
        .iter()
        .filter(|row| matches!(row.target, BookTarget::Unresolved))
        .count();
    for row in &catalogue {
        if !row.errors.is_empty() && !matches!(row.target, BookTarget::Skip) {
            errors.push(format!("Catalogue row {} has invalid required data. Correct the source or explicitly skip the row.", row.source.row_number));
        }
    }
    let counts = ImportCounts {
        catalogue: catalogue.len(),
        historical_loans: history.len(),
        title_reviews: title_reviews.len(),
        owned_yes: catalogue
            .iter()
            .filter(|row| row.book.owned == Some(true))
            .count(),
        currently_borrowed: catalogue
            .iter()
            .filter(|row| row.reading_status == Some(ReadingStatus::CurrentlyBorrowed))
            .count(),
        favourites_yes: catalogue
            .iter()
            .filter(|row| row.favourite == Some(true))
            .count(),
        mom_memberships: catalogue
            .iter()
            .filter(|row| row.mom_rating.is_some())
            .count(),
    };
    let preserved_manual_values = catalogue.iter().map(|row| row.preserve_fields.len()).sum();
    Ok(ImportPreview {
        source_name: workbook.source_name.clone(),
        source_hash: workbook.source_hash.clone(),
        review_hash,
        date_1904: workbook.date_1904,
        draft,
        sheets,
        catalogue,
        history,
        title_reviews,
        fields,
        counts,
        warnings,
        blocking_errors: errors,
        missing_sheets,
        requires_title_decisions,
        preserved_manual_values,
        sources,
        already_imported_batch_id,
    })
}

fn previous_catalogue_target(
    identities: &[CatalogueIdentity],
    source: &SourceRow,
    book: &BookDraft,
    original: Option<&str>,
) -> (Option<BookTarget>, Vec<String>) {
    if let Some(previous) = identities
        .iter()
        .find(|identity| identity.source_row_key == source.key)
    {
        return (
            Some(if previous.skipped {
                BookTarget::Skip
            } else {
                previous
                    .book_id
                    .as_ref()
                    .map(|id| BookTarget::Existing {
                        book_id: id.clone(),
                    })
                    .unwrap_or(BookTarget::Unresolved)
            }),
            previous.book_id.iter().cloned().collect(),
        );
    }
    let anchors: Vec<_> = identities
        .iter()
        .filter(|identity| {
            normalize(identity.source_author.as_deref().unwrap_or(""))
                == normalize(book.authors.as_deref().unwrap_or(""))
                && (original.is_some_and(|original| {
                    identity
                        .original_title
                        .as_deref()
                        .is_some_and(|previous| normalize(previous) == normalize(original))
                }) || normalize(&identity.source_title) == normalize(&book.title))
        })
        .collect();
    let ids: HashSet<_> = anchors
        .iter()
        .filter_map(|identity| identity.book_id.clone())
        .collect();
    if ids.len() == 1 && anchors.iter().all(|identity| !identity.skipped) {
        let id = ids.iter().next().cloned().unwrap_or_default();
        return (
            Some(BookTarget::Existing {
                book_id: id.clone(),
            }),
            vec![id],
        );
    }
    if !anchors.is_empty() {
        return (Some(BookTarget::Unresolved), ids.into_iter().collect());
    }
    let candidates: Vec<_> = identities
        .iter()
        .filter(|identity| identity.source_row_number == source.row_number as i64)
        .filter_map(|identity| identity.book_id.clone())
        .collect();
    if candidates.is_empty() {
        (None, Vec::new())
    } else {
        (Some(BookTarget::Unresolved), candidates)
    }
}

pub async fn initial_draft(
    pool: &SqlitePool,
    source_name: &str,
    token: &str,
) -> Result<ImportReviewDraft, AppError> {
    let mut draft = ImportReviewDraft::initial(token);
    let sources = crate::persistence::import::sources(pool).await?;
    let people = household::list(pool).await?;
    let matching: Vec<_> = sources
        .iter()
        .filter(|source| normalize(&source.last_filename) == normalize(source_name))
        .collect();
    if matching.len() == 1 {
        let source = matching[0];
        draft.source_origin_id = Some(source.id.clone());
        if people
            .people
            .iter()
            .any(|person| person.id == source.carly_person_id && person.archived_at.is_none())
        {
            draft.people.carly = PersonMapping::Existing {
                id: source.carly_person_id.clone(),
            };
        }
        if let Some(id) = &source.mom_person_id {
            if people
                .people
                .iter()
                .any(|person| &person.id == id && person.archived_at.is_none())
            {
                draft.people.mom = Some(PersonMapping::Existing { id: id.clone() });
            }
        }
    }
    Ok(draft)
}

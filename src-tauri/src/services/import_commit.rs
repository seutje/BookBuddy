use crate::{
    domain::{
        self,
        books::{normalized, BookDraft, PersonalDraft},
        custom_fields::{FieldDefinition, FieldDraft, FieldOption, FieldSettings, Kind, Scope},
        import::*,
        AppError,
    },
    persistence::{books, custom_fields, household, import as repository, snapshots},
    services::{import_preview, xlsx_reader},
};
use serde_json::{json, Value};
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

struct PreparedField {
    definition: FieldDefinition,
    options: HashMap<String, String>,
}

async fn person(
    transaction: &mut Transaction<'_, Sqlite>,
    mapping: &PersonMapping,
    summary: &mut ImportSummary,
) -> Result<String, AppError> {
    match mapping {
        PersonMapping::Existing { id } => {
            domain::durable_id(id)?;
            household::require_active(transaction, id).await?;
            Ok(id.clone())
        }
        PersonMapping::New { name } => {
            let name = domain::person_name(name)?;
            let id = uuid::Uuid::new_v4().to_string();
            household::create(transaction, &id, &name).await?;
            summary.people_created += 1;
            Ok(id)
        }
    }
}
async fn personal(
    transaction: &mut Transaction<'_, Sqlite>,
    person: &str,
    book: &str,
    incoming: PersonalDraft,
    summary: &mut ImportSummary,
) -> Result<(), AppError> {
    let previous = repository::membership(transaction, person, book).await?;
    if previous.is_none() {
        books::ensure_membership(transaction, person, book).await?;
        summary.memberships_created += 1;
    }
    let mut current = previous.unwrap_or_default();
    let mut changed = false;
    if current.reading_status.is_none() && incoming.reading_status.is_some() {
        current.reading_status = incoming.reading_status;
        changed = true;
    }
    if current.rating.is_none() && incoming.rating.is_some() {
        current.rating = incoming.rating;
        changed = true;
    }
    // The workbook has no personal list or personal-note column. Neither is inferred.
    if changed {
        books::update_personal(transaction, person, book, &current).await?;
    }
    Ok(())
}

async fn prepare_fields(
    transaction: &mut Transaction<'_, Sqlite>,
    preview: &ImportPreview,
    summary: &mut ImportSummary,
) -> Result<HashMap<String, PreparedField>, AppError> {
    let mut prepared = HashMap::new();
    let definitions = custom_fields::definitions_in(transaction).await?;
    let mut next_order = definitions
        .iter()
        .map(|definition| definition.draft.order_index)
        .max()
        .unwrap_or(-1)
        + 1;
    for proposed in &preview.fields {
        let (id, mut draft, is_new) = if let Some(id) = &proposed.existing_field_id {
            let definition = custom_fields::definition_in(transaction, id).await?;
            (id.clone(), definition.draft, false)
        } else {
            let kind = match proposed.kind.as_str() {
                "select" => Kind::Select,
                "boolean" => Kind::Boolean,
                "text" => Kind::Text,
                _ => return Err(AppError::validation("Unsupported source-field mapping.")),
            };
            let scope = match proposed.scope.as_str() {
                "book" => Scope::Book,
                "person_book" => Scope::PersonBook,
                _ => return Err(AppError::validation("Unsupported source-field scope.")),
            };
            let draft = FieldDraft { scope, kind, label: proposed.label.clone(), description: "Imported from the family library workbook. Original cell labels and values are retained in Import History.".into(), settings: FieldSettings { multiline: kind == Kind::Text, ..Default::default() }, default_value: None, options: Vec::new(), order_index: next_order, active: true, show_detail: true, show_in_rows: false, show_in_filters: true };
            next_order += 1;
            (uuid::Uuid::new_v4().to_string(), draft, true)
        };
        let mut options = HashMap::new();
        let mut options_changed = false;
        if draft.kind == Kind::Select {
            for raw_label in &proposed.choices {
                let Ok(label) = domain::person_name(raw_label) else {
                    continue;
                };
                let key = normalized(raw_label);
                if options.contains_key(&key) {
                    continue;
                }
                let mapped = repository::option_mapping(
                    transaction,
                    &summary.origin_id,
                    &proposed.source_key,
                    &id,
                    raw_label,
                )
                .await?;
                let option_id = if let Some(mapped) = mapped {
                    mapped
                } else if let Some(option) = draft
                    .options
                    .iter()
                    .find(|option| option.active && normalized(&option.label) == normalized(&label))
                    .or_else(|| {
                        draft
                            .options
                            .iter()
                            .find(|option| normalized(&option.label) == normalized(&label))
                    })
                {
                    option.id.clone()
                } else {
                    let option_id = uuid::Uuid::new_v4().to_string();
                    draft.options.push(FieldOption {
                        id: option_id.clone(),
                        label,
                        active: true,
                    });
                    options_changed = true;
                    option_id
                };
                options.insert(key, option_id);
            }
        }
        let draft = draft.validate()?;
        if is_new || options_changed {
            custom_fields::save_definition(transaction, &id, &draft).await?;
        }
        if is_new {
            summary.fields_created += 1;
        }
        repository::map_field(transaction, &summary.origin_id, &proposed.source_key, &id).await?;
        for raw_label in &proposed.choices {
            if let Some(option_id) = options.get(&normalized(raw_label)) {
                repository::map_option(
                    transaction,
                    &summary.origin_id,
                    &proposed.source_key,
                    &id,
                    raw_label,
                    option_id,
                )
                .await?;
            }
        }
        let definition = custom_fields::definition_in(transaction, &id).await?;
        prepared.insert(
            proposed.source_key.clone(),
            PreparedField {
                definition,
                options,
            },
        );
    }
    Ok(prepared)
}

fn incoming_custom(row: &CatalogueRow, key: &str, field: &PreparedField) -> Option<Value> {
    match key {
        "favourite" => row.favourite.map(Value::Bool),
        "renewed_to_death" => row.renewed_to_death.map(Value::Bool),
        "age_range" => row.age_range.clone().map(Value::String),
        "reading_effort" => row.reading_effort.clone().map(Value::String),
        "format" | "buy_status" => {
            let raw = if key == "format" {
                row.format.as_deref()
            } else {
                row.buy_status.as_deref()
            }?;
            field
                .options
                .get(&normalized(raw))
                .cloned()
                .map(Value::String)
        }
        _ => None,
    }
}
async fn custom_values(
    transaction: &mut Transaction<'_, Sqlite>,
    row: &CatalogueRow,
    book: &str,
    carly: &str,
    fields: &HashMap<String, PreparedField>,
    summary: &mut ImportSummary,
) -> Result<(), AppError> {
    let personal_id = repository::membership_id(transaction, carly, book).await?;
    let shared = custom_fields::values_in(transaction, Scope::Book, book).await?;
    let personal = custom_fields::values_in(transaction, Scope::PersonBook, &personal_id).await?;
    for (key, field) in fields {
        let Some(value) = incoming_custom(row, key, field) else {
            continue;
        };
        let (values, target) = if field.definition.draft.scope == Scope::Book {
            (&shared, book)
        } else {
            (&personal, personal_id.as_str())
        };
        if values.contains_key(&field.definition.id) {
            continue;
        }
        // Unsupported source values remain in provenance, with preview warnings.
        if field
            .definition
            .draft
            .validate_value(&value, false)
            .is_err()
        {
            continue;
        }
        custom_fields::set_value(
            transaction,
            field.definition.draft.scope,
            target,
            &field.definition,
            Some(value),
        )
        .await?;
        summary.custom_values_written += 1;
    }
    Ok(())
}

async fn target_book(
    transaction: &mut Transaction<'_, Sqlite>,
    target: &BookTarget,
    catalogue: &HashMap<String, Option<String>>,
    incoming: BookDraft,
    added: &mut HashSet<String>,
) -> Result<Option<String>, AppError> {
    match target {
        BookTarget::Skip | BookTarget::Unresolved => Ok(None),
        BookTarget::Existing { book_id } => {
            domain::durable_id(book_id)?;
            books::require_book(transaction, book_id).await?;
            Ok(Some(book_id.clone()))
        }
        BookTarget::Catalogue { row_key } => catalogue.get(row_key).cloned().ok_or_else(|| {
            AppError::validation("A source-row match refers to an unavailable catalogue row.")
        }),
        BookTarget::New => {
            let incoming = incoming.validate()?;
            let id = uuid::Uuid::new_v4().to_string();
            books::create(transaction, &id, &incoming).await?;
            added.insert(id.clone());
            Ok(Some(id))
        }
    }
}
fn warnings_for(preview: &ImportPreview, row: &SourceRow) -> Vec<ImportWarning> {
    preview
        .warnings
        .iter()
        .filter(|warning| {
            warning.sheet.as_deref() == Some(&row.sheet) && warning.row == Some(row.row_number)
        })
        .cloned()
        .collect()
}
async fn apply_title(
    transaction: &mut Transaction<'_, Sqlite>,
    task_id: &str,
    book_id: Option<&str>,
    confirmed: &str,
    updated: &mut HashSet<String>,
) -> Result<(), AppError> {
    let title = BookDraft {
        title: confirmed.into(),
        ..Default::default()
    }
    .validate()?
    .title;
    let Some(book_id) = book_id else {
        return Err(AppError::validation(
            "Link the verified title to a book before applying its correction.",
        ));
    };
    let mut current = books::book_in(transaction, book_id).await?.data;
    current.title = title.clone();
    books::update(transaction, book_id, &current.validate()?).await?;
    repository::resolve_task(transaction, task_id, Some(book_id), Some(&title)).await?;
    updated.insert(book_id.into());
    Ok(())
}
fn validate_ready(preview: &ImportPreview) -> Result<(), AppError> {
    if !preview.blocking_errors.is_empty() || preview.requires_title_decisions > 0 {
        return Err(AppError::validation("Resolve the blocking source issues and catalogue matches, then refresh the preview before importing."));
    }
    if !preview.warnings.is_empty() && !preview.draft.acknowledge_warnings {
        return Err(AppError::validation(
            "Review and acknowledge the source warnings before importing.",
        ));
    }
    if preview
        .missing_sheets
        .iter()
        .any(|sheet| sheet == HISTORY || sheet == TITLES)
        && !preview.draft.acknowledge_missing_sheets
    {
        return Err(AppError::validation(
            "Explicitly acknowledge the missing source-data sheets before importing.",
        ));
    }
    let keys: HashSet<&str> = preview
        .catalogue
        .iter()
        .map(|row| row.source.key.as_str())
        .chain(preview.history.iter().map(|row| row.source.key.as_str()))
        .chain(
            preview
                .title_reviews
                .iter()
                .map(|row| row.source.key.as_str()),
        )
        .collect();
    let mut seen = HashSet::new();
    for decision in &preview.draft.decisions {
        if !keys.contains(decision.row_key.as_str()) || !seen.insert(&decision.row_key) {
            return Err(AppError::validation(
                "Import decisions contain an unknown or repeated source-row identity.",
            ));
        }
    }
    for row in &preview.catalogue {
        if matches!(row.target, BookTarget::Catalogue { .. }) {
            return Err(AppError::validation("Catalogue rows must create a separate book, match an existing book, or be explicitly skipped."));
        }
    }
    Ok(())
}

/// Accepts only a previously parsed local selection. The command layer owns path authorization.
pub async fn commit(
    pool: &SqlitePool,
    data_directory: &Path,
    path: &Path,
    workbook: &WorkbookData,
    draft: ImportReviewDraft,
    reviewed_hash: &str,
) -> Result<ImportSummary, AppError> {
    let current_bytes = xlsx_reader::read_bytes(path)?;
    if xlsx_reader::hash(&current_bytes) != workbook.source_hash {
        return Err(AppError::validation(
            "The workbook changed after inspection. Choose it again and review a fresh preview.",
        ));
    }
    if let Some(batch) = repository::batch_by_hash(pool, &workbook.source_hash).await? {
        let original: ImportSummary = serde_json::from_str(&batch.summary_json).map_err(|_| {
            AppError::validation(
                "The saved import summary could not be read. Keep the database for recovery.",
            )
        })?;
        return Ok(ImportSummary {
            batch_id: batch.id,
            origin_id: batch.origin_id,
            already_imported: true,
            carly_person_id: original.carly_person_id,
            ..Default::default()
        });
    }
    let initial = import_preview::preview(pool, workbook, draft.clone()).await?;
    if initial.review_hash != reviewed_hash {
        return Err(AppError::validation(
            "The library or mappings changed after review. Refresh the preview before importing.",
        ));
    }
    validate_ready(&initial)?;
    let backup = snapshots::snapshot(pool, &data_directory.join("backups"), "pre-import").await?;
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let preview = import_preview::preview(pool, workbook, draft).await?;
    if preview.review_hash != reviewed_hash {
        return Err(AppError::validation(
            "The library changed during review. Refresh the preview before importing.",
        ));
    }
    validate_ready(&preview)?;
    let mut summary = ImportSummary {
        batch_id: uuid::Uuid::new_v4().to_string(),
        origin_id: preview
            .draft
            .source_origin_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        warning_count: preview.warnings.len(),
        backup_name: backup
            .file_name()
            .map(|name| name.to_string_lossy().into_owned()),
        ..Default::default()
    };
    let carly = person(&mut transaction, &preview.draft.people.carly, &mut summary).await?;
    let mom = match &preview.draft.people.mom {
        Some(mapping) => Some(person(&mut transaction, mapping, &mut summary).await?),
        None => None,
    };
    if mom.as_deref() == Some(&carly) {
        return Err(AppError::validation(
            "Carly and Mom must map to distinct people.",
        ));
    }
    summary.carly_person_id = carly.clone();
    repository::save_source(
        &mut transaction,
        &summary.origin_id,
        &preview.source_name,
        &carly,
        mom.as_deref(),
    )
    .await?;
    let reference_sheets: Vec<_> = workbook
        .sheets
        .iter()
        .filter(|sheet| {
            ![CATALOGUE, HISTORY, TITLES]
                .iter()
                .any(|name| normalized(name) == normalized(&sheet.name))
        })
        .collect();
    let references = json!({ "date1904": workbook.date_1904, "sheets": preview.sheets, "referenceSheets": reference_sheets, "originalSheets": workbook.sheets });
    repository::insert_batch(&mut transaction, &summary, &preview, &references).await?;
    let prepared = prepare_fields(&mut transaction, &preview, &mut summary).await?;
    let mut resolved_catalogue = HashMap::new();
    let mut source_ids = HashMap::new();
    let mut added = HashSet::new();
    let mut updated = HashSet::new();
    for row in &preview.catalogue {
        let source_id = repository::insert_row(
            &mut transaction,
            &summary.batch_id,
            &row.source,
            &warnings_for(&preview, &row.source),
        )
        .await?;
        summary.source_rows_saved += 1;
        source_ids.insert(row.source.key.clone(), source_id.clone());
        if matches!(row.target, BookTarget::Skip) {
            repository::map_catalogue(
                &mut transaction,
                &summary.origin_id,
                row,
                None,
                &summary.batch_id,
            )
            .await?;
            resolved_catalogue.insert(row.source.key.clone(), None);
            summary.catalogue_rows_skipped += 1;
            continue;
        }
        let book_id = target_book(
            &mut transaction,
            &row.target,
            &resolved_catalogue,
            row.book.clone(),
            &mut added,
        )
        .await?
        .ok_or_else(|| {
            AppError::validation("Resolve the catalogue book match before importing.")
        })?;
        if !added.contains(&book_id) {
            let current = books::book_in(&mut transaction, &book_id).await?;
            let (filled, changed) = fill_book(current.data, &row.book);
            if changed {
                books::update(&mut transaction, &book_id, &filled.validate()?).await?;
                updated.insert(book_id.clone());
            }
        }
        personal(
            &mut transaction,
            &carly,
            &book_id,
            PersonalDraft {
                reading_status: row.reading_status,
                rating: row.carly_rating,
                ..Default::default()
            },
            &mut summary,
        )
        .await?;
        if let (Some(mom), Some(rating)) = (&mom, row.mom_rating) {
            personal(
                &mut transaction,
                mom,
                &book_id,
                PersonalDraft {
                    rating: Some(rating),
                    ..Default::default()
                },
                &mut summary,
            )
            .await?;
        }
        custom_values(
            &mut transaction,
            row,
            &book_id,
            &carly,
            &prepared,
            &mut summary,
        )
        .await?;
        repository::save_baseline(&mut transaction, &summary.origin_id, &book_id, row).await?;
        repository::link_row_book(&mut transaction, &source_id, &book_id).await?;
        repository::map_catalogue(
            &mut transaction,
            &summary.origin_id,
            row,
            Some(&book_id),
            &summary.batch_id,
        )
        .await?;
        resolved_catalogue.insert(row.source.key.clone(), Some(book_id));
    }
    for row in &preview.history {
        let source_id = repository::insert_row(
            &mut transaction,
            &summary.batch_id,
            &row.source,
            &warnings_for(&preview, &row.source),
        )
        .await?;
        summary.source_rows_saved += 1;
        if matches!(row.target, BookTarget::Skip) {
            continue;
        }
        if let Some(loan_id) =
            repository::known_loan(&mut transaction, &summary.origin_id, &row.source.key).await?
        {
            repository::link_row_loan(&mut transaction, &source_id, &loan_id).await?;
            summary.historical_loans_reused += 1;
            continue;
        }
        let book_id = target_book(
            &mut transaction,
            &row.target,
            &resolved_catalogue,
            BookDraft {
                title: row.title.clone(),
                authors: row.author.clone(),
                publication_year: row.publication_year,
                ..Default::default()
            },
            &mut added,
        )
        .await?;
        let loan_id = repository::insert_loan(
            &mut transaction,
            &summary.origin_id,
            row,
            &source_id,
            &carly,
            book_id.as_deref(),
        )
        .await?;
        repository::link_row_loan(&mut transaction, &source_id, &loan_id).await?;
        summary.historical_loans_added += 1;
        if book_id.is_none() {
            summary.unlinked_loans += 1;
        }
    }
    for row in &preview.title_reviews {
        let source_id = repository::insert_row(
            &mut transaction,
            &summary.batch_id,
            &row.source,
            &warnings_for(&preview, &row.source),
        )
        .await?;
        summary.source_rows_saved += 1;
        if matches!(row.target, BookTarget::Skip) {
            continue;
        }
        let known =
            repository::known_task(&mut transaction, &summary.origin_id, &row.identity_key).await?;
        let task_id = if let Some(id) = known {
            id
        } else {
            let book_id = target_book(
                &mut transaction,
                &row.target,
                &resolved_catalogue,
                BookDraft {
                    title: row.working_title.clone(),
                    authors: row.author.clone(),
                    ..Default::default()
                },
                &mut added,
            )
            .await?;
            summary.title_tasks_added += 1;
            repository::insert_task(
                &mut transaction,
                &summary.origin_id,
                row,
                &source_id,
                book_id.as_deref(),
            )
            .await?
        };
        repository::link_row_task(&mut transaction, &source_id, &task_id).await?;
        repository::update_open_task_proposal(&mut transaction, &task_id, row).await?;
        let saved = repository::task_in(&mut transaction, &task_id).await?;
        if row.accept_confirmed_title && saved.state == "open" {
            if let Some(title) = &row.confirmed_title {
                apply_title(
                    &mut transaction,
                    &task_id,
                    saved.book_id.as_deref(),
                    title,
                    &mut updated,
                )
                .await?;
            }
        }
    }
    for row in &preview.catalogue {
        if row.check_title != Some(true) {
            continue;
        }
        let Some(Some(book_id)) = resolved_catalogue.get(&row.source.key) else {
            continue;
        };
        repository::flag_book(&mut transaction, book_id).await?;
        if !repository::has_task_for_title_or_book(
            &mut transaction,
            &summary.origin_id,
            book_id,
            &row.book.title,
        )
        .await?
        {
            let flag = TitleReviewRow {
                source: row.source.clone(),
                identity_key: format!("catalogue-flag:{book_id}"),
                working_title: row.book.title.clone(),
                author: row.book.authors.clone(),
                raw_pdf_title: row.original_title.clone(),
                reason: Some("Source catalogue flagged this title for verification.".into()),
                confirmed_title: None,
                catalogue_candidates: Vec::new(),
                candidates: Vec::new(),
                target: BookTarget::Existing {
                    book_id: book_id.clone(),
                },
                accept_confirmed_title: false,
                existing_task_id: None,
                existing_state: None,
            };
            repository::insert_task(
                &mut transaction,
                &summary.origin_id,
                &flag,
                source_ids
                    .get(&row.source.key)
                    .ok_or_else(|| AppError::validation("Missing source provenance."))?,
                Some(book_id),
            )
            .await?;
            summary.title_tasks_added += 1;
        }
    }
    household::select(&mut transaction, Some(&carly)).await?;
    summary.books_added = added.len();
    summary.books_updated = updated.difference(&added).count();
    summary.unresolved_title_tasks = repository::unresolved_count(&mut transaction).await? as usize;
    repository::finish_batch(&mut transaction, &summary).await?;
    if xlsx_reader::hash(&xlsx_reader::read_bytes(path)?) != workbook.source_hash {
        return Err(AppError::validation("The workbook changed while the import was being prepared. No library changes were committed. Choose the file again and review it."));
    }
    transaction.commit().await?;
    Ok(summary)
}

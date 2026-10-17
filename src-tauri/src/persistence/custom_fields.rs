use crate::domain::{
    books::{normalized, BookDetail},
    custom_fields::*,
    AppError,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::collections::HashMap;

#[derive(sqlx::FromRow)]
struct RawDefinition {
    id: String,
    scope: Scope,
    kind: Kind,
    label: String,
    description: String,
    settings_json: String,
    default_json: Option<String>,
    order_index: i64,
    active: bool,
    show_detail: bool,
    show_in_rows: bool,
    show_in_filters: bool,
    revision: i64,
    value_count: i64,
}

fn decode<T: DeserializeOwned>(raw: &str) -> Result<T, AppError> {
    serde_json::from_str(raw).map_err(|_| {
        AppError::validation(
            "A saved custom field could not be decoded. Keep the database for recovery.",
        )
    })
}

pub async fn definitions_in(
    transaction: &mut Transaction<'_, Sqlite>,
) -> Result<Vec<FieldDefinition>, AppError> {
    let raw = sqlx::query_as::<_, RawDefinition>("SELECT d.*, (SELECT COUNT(*) FROM book_custom_field_value WHERE field_id = d.id) + (SELECT COUNT(*) FROM person_book_custom_field_value WHERE field_id = d.id) AS value_count FROM custom_field_definition d ORDER BY order_index, id")
        .fetch_all(&mut **transaction).await?;
    let mut definitions = Vec::new();
    for row in raw {
        let options = sqlx::query_as::<_, FieldOption>("SELECT id, label, active FROM custom_field_option WHERE field_id = ? ORDER BY order_index, id")
            .bind(&row.id).fetch_all(&mut **transaction).await?;
        let settings = decode(&row.settings_json)?;
        let default_value = row.default_json.as_deref().map(decode).transpose()?;
        definitions.push(FieldDefinition {
            id: row.id,
            draft: FieldDraft {
                scope: row.scope,
                kind: row.kind,
                label: row.label,
                description: row.description,
                settings,
                default_value,
                options,
                order_index: row.order_index,
                active: row.active,
                show_detail: row.show_detail,
                show_in_rows: row.show_in_rows,
                show_in_filters: row.show_in_filters,
            },
            revision: row.revision,
            value_count: row.value_count,
        });
    }
    Ok(definitions)
}

pub async fn definitions(pool: &SqlitePool) -> Result<Vec<FieldDefinition>, AppError> {
    let mut transaction = pool.begin().await?;
    let result = definitions_in(&mut transaction).await?;
    transaction.commit().await?;
    Ok(result)
}

pub async fn definition_in(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<FieldDefinition, AppError> {
    definitions_in(transaction)
        .await?
        .into_iter()
        .find(|definition| definition.id == id)
        .ok_or_else(|| AppError::validation("That custom field does not exist."))
}

pub async fn save_definition(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
    draft: &FieldDraft,
) -> Result<(), AppError> {
    let duplicates: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM custom_field_definition WHERE active = 1 AND scope = ? AND normalized_label = ? AND id != ?")
        .bind(draft.scope).bind(normalized(&draft.label)).bind(id).fetch_one(&mut **transaction).await?;
    if draft.active && duplicates != 0 {
        return Err(AppError::validation(
            "An active field with that label already exists in this scope.",
        ));
    }
    for option in &draft.options {
        let owner: Option<String> =
            sqlx::query_scalar("SELECT field_id FROM custom_field_option WHERE id = ?")
                .bind(&option.id)
                .fetch_optional(&mut **transaction)
                .await?;
        if owner.is_some_and(|owner| owner != id) {
            return Err(AppError::validation(
                "A choice ID belongs to another field.",
            ));
        }
    }
    let settings = serde_json::to_string(&draft.settings)
        .map_err(|_| AppError::validation("Invalid field settings."))?;
    let default = draft.default_value.as_ref().map(Value::to_string);
    sqlx::query("INSERT INTO custom_field_definition (id, scope, kind, label, normalized_label, description, settings_json, default_json, order_index, active, show_detail, show_in_rows, show_in_filters) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET scope = excluded.scope, kind = excluded.kind, label = excluded.label, normalized_label = excluded.normalized_label, description = excluded.description, settings_json = excluded.settings_json, default_json = excluded.default_json, order_index = excluded.order_index, active = excluded.active, show_detail = excluded.show_detail, show_in_rows = excluded.show_in_rows, show_in_filters = excluded.show_in_filters, revision = custom_field_definition.revision + 1")
        .bind(id).bind(draft.scope).bind(draft.kind).bind(&draft.label).bind(normalized(&draft.label)).bind(&draft.description).bind(settings).bind(default).bind(draft.order_index).bind(draft.active).bind(draft.show_detail).bind(draft.show_in_rows).bind(draft.show_in_filters).execute(&mut **transaction).await?;
    // Missing choices are retired, never deleted or reused for another field.
    sqlx::query("UPDATE custom_field_option SET active = 0 WHERE field_id = ?")
        .bind(id)
        .execute(&mut **transaction)
        .await?;
    for (index, option) in draft.options.iter().enumerate() {
        sqlx::query("INSERT INTO custom_field_option (id, field_id, label, order_index, active) VALUES (?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET label = excluded.label, order_index = excluded.order_index, active = excluded.active")
            .bind(&option.id).bind(id).bind(&option.label).bind(index as i64).bind(option.active).execute(&mut **transaction).await?;
    }
    Ok(())
}

pub async fn clear_values(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM book_custom_field_value WHERE field_id = ?")
        .bind(id)
        .execute(&mut **transaction)
        .await?;
    sqlx::query("DELETE FROM person_book_custom_field_value WHERE field_id = ?")
        .bind(id)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

pub async fn reorder(
    transaction: &mut Transaction<'_, Sqlite>,
    ids: &[String],
) -> Result<(), AppError> {
    for (index, id) in ids.iter().enumerate() {
        sqlx::query("UPDATE custom_field_definition SET order_index = ?, revision = revision + 1 WHERE id = ?").bind(index as i64).bind(id).execute(&mut **transaction).await?;
    }
    Ok(())
}

pub async fn resolve_target(
    transaction: &mut Transaction<'_, Sqlite>,
    target: &ValueTarget,
    writing: bool,
) -> Result<(Scope, String), AppError> {
    target.validate()?;
    match target {
        ValueTarget::Book { book_id } => {
            super::books::require_book(transaction, book_id).await?;
            Ok((Scope::Book, book_id.clone()))
        }
        ValueTarget::PersonBook { person_id, book_id } => {
            if writing {
                super::household::require_active(transaction, person_id).await?;
            }
            let id: Option<String> = sqlx::query_scalar(
                "SELECT id FROM person_book WHERE person_id = ? AND book_id = ?",
            )
            .bind(person_id)
            .bind(book_id)
            .fetch_optional(&mut **transaction)
            .await?;
            Ok((
                Scope::PersonBook,
                id.ok_or_else(|| {
                    AppError::validation(
                        "Assign this book to that person before editing personal fields.",
                    )
                })?,
            ))
        }
    }
}

pub async fn values_in(
    transaction: &mut Transaction<'_, Sqlite>,
    scope: Scope,
    target_id: &str,
) -> Result<HashMap<String, Value>, AppError> {
    let rows: Vec<(String, String)> = match scope {
        Scope::Book => sqlx::query_as("SELECT field_id, value_json FROM book_custom_field_value WHERE book_id = ?").bind(target_id).fetch_all(&mut **transaction).await?,
        Scope::PersonBook => sqlx::query_as("SELECT field_id, value_json FROM person_book_custom_field_value WHERE person_book_id = ?").bind(target_id).fetch_all(&mut **transaction).await?,
    };
    rows.into_iter()
        .map(|(id, raw)| Ok((id, decode(&raw)?)))
        .collect()
}

pub async fn set_value(
    transaction: &mut Transaction<'_, Sqlite>,
    scope: Scope,
    target_id: &str,
    definition: &FieldDefinition,
    value: Option<Value>,
) -> Result<(), AppError> {
    let json = value
        .as_ref()
        .map(|value| canonical_value(&definition.draft, value))
        .transpose()?;
    match (scope, json) {
        (Scope::Book, Some(json)) => {
            sqlx::query("INSERT INTO book_custom_field_value (book_id, field_id, value_json) VALUES (?, ?, ?) ON CONFLICT(book_id, field_id) DO UPDATE SET value_json = excluded.value_json, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')").bind(target_id).bind(&definition.id).bind(json).execute(&mut **transaction).await?;
        }
        (Scope::PersonBook, Some(json)) => {
            sqlx::query("INSERT INTO person_book_custom_field_value (person_book_id, field_id, value_json) VALUES (?, ?, ?) ON CONFLICT(person_book_id, field_id) DO UPDATE SET value_json = excluded.value_json, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')").bind(target_id).bind(&definition.id).bind(json).execute(&mut **transaction).await?;
        }
        (Scope::Book, None) => {
            sqlx::query("DELETE FROM book_custom_field_value WHERE book_id = ? AND field_id = ?")
                .bind(target_id)
                .bind(&definition.id)
                .execute(&mut **transaction)
                .await?;
        }
        (Scope::PersonBook, None) => {
            sqlx::query("DELETE FROM person_book_custom_field_value WHERE person_book_id = ? AND field_id = ?").bind(target_id).bind(&definition.id).execute(&mut **transaction).await?;
        }
    }
    sqlx::query("UPDATE custom_field_definition SET revision = revision + 1 WHERE id = ?")
        .bind(&definition.id)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

pub async fn decorate(
    transaction: &mut Transaction<'_, Sqlite>,
    rows: &mut [BookDetail],
    person_id: Option<&str>,
) -> Result<(), AppError> {
    let definitions = definitions_in(transaction).await?;
    let book_values: Vec<(String, String, String)> =
        sqlx::query_as("SELECT book_id, field_id, value_json FROM book_custom_field_value")
            .fetch_all(&mut **transaction)
            .await?;
    let personal_values: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT person_book_id, field_id, value_json FROM person_book_custom_field_value",
    )
    .fetch_all(&mut **transaction)
    .await?;
    let books: HashMap<(String, String), Value> = book_values
        .into_iter()
        .map(|(target, field, value)| Ok(((target, field), decode(&value)?)))
        .collect::<Result<_, AppError>>()?;
    let personal: HashMap<(String, String), Value> = personal_values
        .into_iter()
        .map(|(target, field, value)| Ok(((target, field), decode(&value)?)))
        .collect::<Result<_, AppError>>()?;
    for row in rows {
        for definition in definitions
            .iter()
            .filter(|definition| definition.draft.active)
        {
            let draft = &definition.draft;
            match draft.scope {
                Scope::Book => {
                    let value = books.get(&(row.book.id.clone(), definition.id.clone()));
                    if draft.kind == Kind::Text {
                        if let Some(text) = value.and_then(Value::as_str) {
                            row.custom_search.push(text.to_owned());
                        }
                    }
                    if draft.show_in_rows {
                        row.row_fields.push(CustomDisplay {
                            field_id: definition.id.clone(),
                            label: draft.label.clone(),
                            person_name: None,
                            display: display_value(draft, value),
                        });
                    }
                }
                Scope::PersonBook => {
                    for membership in row
                        .memberships
                        .iter()
                        .filter(|membership| person_id.is_none_or(|id| id == membership.person_id))
                    {
                        let value = personal.get(&(membership.id.clone(), definition.id.clone()));
                        if draft.kind == Kind::Text {
                            if let Some(text) = value.and_then(Value::as_str) {
                                row.custom_search.push(text.to_owned());
                            }
                        }
                        if draft.show_in_rows {
                            row.row_fields.push(CustomDisplay {
                                field_id: definition.id.clone(),
                                label: draft.label.clone(),
                                person_name: Some(membership.person_name.clone()),
                                display: display_value(draft, value),
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

use crate::{
    domain::{custom_fields::*, durable_id, AppError},
    persistence::{custom_fields as repository, household},
};
use serde_json::Value;
use sqlx::SqlitePool;
use std::collections::HashSet;

pub async fn list(pool: &SqlitePool) -> Result<Vec<FieldDefinition>, AppError> {
    repository::definitions(pool).await
}

pub async fn create(pool: &SqlitePool, draft: FieldDraft) -> Result<String, AppError> {
    let draft = draft.validate()?;
    let id = uuid::Uuid::new_v4().to_string();
    let mut transaction = household::begin(pool).await?;
    repository::save_definition(&mut transaction, &id, &draft).await?;
    transaction.commit().await?;
    Ok(id)
}

pub async fn preview_change(
    pool: &SqlitePool,
    id: &str,
    draft: FieldDraft,
) -> Result<ChangePreview, AppError> {
    durable_id(id)?;
    let draft = draft.validate()?;
    let mut transaction = household::begin(pool).await?;
    let old = repository::definition_in(&mut transaction, id).await?;
    let result = preview(&old, &draft)?;
    transaction.commit().await?;
    Ok(result)
}

pub async fn update(
    pool: &SqlitePool,
    id: &str,
    draft: FieldDraft,
    token: &str,
    confirmed: bool,
) -> Result<(), AppError> {
    durable_id(id)?;
    let draft = draft.validate()?;
    let mut transaction = household::begin(pool).await?;
    let old = repository::definition_in(&mut transaction, id).await?;
    let impact = preview(&old, &draft)?;
    if token != impact.token {
        return Err(AppError::validation(
            "The field or its values changed after preview. Review the change again.",
        ));
    }
    if impact.requires_confirmation && !confirmed {
        return Err(AppError::validation(
            "Review and explicitly confirm this field change first.",
        ));
    }
    if impact.cleared_value_count > 0 {
        repository::clear_values(&mut transaction, id).await?;
    }
    repository::save_definition(&mut transaction, id, &draft).await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn reorder(pool: &SqlitePool, ids: Vec<String>) -> Result<(), AppError> {
    let mut transaction = household::begin(pool).await?;
    let definitions = repository::definitions_in(&mut transaction).await?;
    let expected: HashSet<_> = definitions
        .iter()
        .map(|definition| definition.id.as_str())
        .collect();
    let supplied: HashSet<_> = ids.iter().map(String::as_str).collect();
    if supplied != expected || supplied.len() != ids.len() {
        return Err(AppError::validation(
            "Reorder must include every field exactly once. Refresh the field list.",
        ));
    }
    repository::reorder(&mut transaction, &ids).await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn values(pool: &SqlitePool, target: ValueTarget) -> Result<Vec<FieldValue>, AppError> {
    let mut transaction = household::begin(pool).await?;
    let (scope, target_id) = repository::resolve_target(&mut transaction, &target, false).await?;
    let definitions = repository::definitions_in(&mut transaction).await?;
    let mut values = repository::values_in(&mut transaction, scope, &target_id).await?;
    transaction.commit().await?;
    Ok(definitions
        .into_iter()
        .filter(|definition| definition.draft.scope == scope && definition.draft.active)
        .map(|definition| {
            let value = values.remove(&definition.id);
            FieldValue { definition, value }
        })
        .collect())
}

pub async fn set_value(
    pool: &SqlitePool,
    target: ValueTarget,
    field_id: &str,
    value: Option<Value>,
) -> Result<(), AppError> {
    durable_id(field_id)?;
    let mut transaction = household::begin(pool).await?;
    let (scope, target_id) = repository::resolve_target(&mut transaction, &target, true).await?;
    let definition = repository::definition_in(&mut transaction, field_id).await?;
    if !definition.draft.active || definition.draft.scope != scope {
        return Err(AppError::validation(
            "This field is hidden or belongs to another scope.",
        ));
    }
    if let Some(value) = &value {
        definition.draft.validate_value(value, false)?;
    }
    repository::set_value(&mut transaction, scope, &target_id, &definition, value).await?;
    transaction.commit().await?;
    Ok(())
}

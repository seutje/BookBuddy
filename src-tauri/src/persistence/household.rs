use crate::domain::{AppError, Household, Person};
use sqlx::{Sqlite, SqlitePool, Transaction};

pub async fn begin(pool: &SqlitePool) -> Result<Transaction<'_, Sqlite>, AppError> {
    Ok(pool.begin().await?)
}

pub async fn snapshot(pool: &SqlitePool) -> Result<Household, AppError> {
    let mut transaction = begin(pool).await?;
    let people = sqlx::query_as::<_, Person>(
        "SELECT id, display_name, archived_at, created_at FROM person ORDER BY created_at, id",
    )
    .fetch_all(&mut *transaction)
    .await?;
    let current: Option<String> =
        sqlx::query_scalar("SELECT value_json FROM app_settings WHERE key = 'current_person'")
            .fetch_optional(&mut *transaction)
            .await?;
    let current_person_id: Option<String> = current
        .map(|value| serde_json::from_str::<Option<String>>(&value))
        .transpose()
        .map_err(|_| AppError::startup())?
        .flatten();
    // Never expose an archived or missing person as the editing context.
    let current_person_id = current_person_id.filter(|id| {
        people
            .iter()
            .any(|person| &person.id == id && person.archived_at.is_none())
    });
    transaction.commit().await?;
    Ok(Household {
        people,
        current_person_id,
    })
}

pub async fn require_active(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<(), AppError> {
    let found: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM person WHERE id = ? AND archived_at IS NULL")
            .bind(id)
            .fetch_one(&mut **transaction)
            .await?;
    if found == 0 {
        return Err(AppError::not_found());
    }
    Ok(())
}

pub async fn select(
    transaction: &mut Transaction<'_, Sqlite>,
    id: Option<&str>,
) -> Result<(), AppError> {
    if let Some(id) = id {
        require_active(transaction, id).await?;
    }
    let json = serde_json::to_string(&id)
        .map_err(|_| AppError::validation("Invalid person selection."))?;
    sqlx::query("INSERT INTO app_settings (key, value_json) VALUES ('current_person', ?) ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json")
        .bind(json).execute(&mut **transaction).await?;
    Ok(())
}

pub async fn create(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
    name: &str,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO person (id, display_name) VALUES (?, ?)")
        .bind(id)
        .bind(name)
        .execute(&mut **transaction)
        .await?;
    select(transaction, Some(id)).await
}

pub async fn rename(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
    name: &str,
) -> Result<(), AppError> {
    require_active(transaction, id).await?;
    sqlx::query("UPDATE person SET display_name = ? WHERE id = ?")
        .bind(name)
        .bind(id)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

pub async fn archive(transaction: &mut Transaction<'_, Sqlite>, id: &str) -> Result<(), AppError> {
    require_active(transaction, id).await?;
    sqlx::query(
        "UPDATE person SET archived_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(id)
    .execute(&mut **transaction)
    .await?;
    let selected: Option<String> =
        sqlx::query_scalar("SELECT value_json FROM app_settings WHERE key = 'current_person'")
            .fetch_optional(&mut **transaction)
            .await?;
    if selected.as_deref()
        == Some(
            serde_json::to_string(id)
                .map_err(|_| AppError::validation("Invalid person selection."))?
                .as_str(),
        )
    {
        // Archiving the selected person returns to Everyone, never another person's editor.
        select(transaction, None).await?;
    }
    Ok(())
}

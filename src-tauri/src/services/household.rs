use crate::{
    domain::{self, AppError, Household},
    persistence::household as repository,
};
use sqlx::SqlitePool;

pub async fn list(pool: &SqlitePool) -> Result<Household, AppError> {
    repository::snapshot(pool).await
}

pub async fn create(pool: &SqlitePool, raw_name: &str) -> Result<(), AppError> {
    let name = domain::person_name(raw_name)?;
    let id = uuid::Uuid::new_v4().to_string();
    let mut transaction = repository::begin(pool).await?;
    repository::create(&mut transaction, &id, &name).await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn rename(pool: &SqlitePool, id: &str, raw_name: &str) -> Result<(), AppError> {
    domain::durable_id(id)?;
    let name = domain::person_name(raw_name)?;
    let mut transaction = repository::begin(pool).await?;
    repository::rename(&mut transaction, id, &name).await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn archive(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    domain::durable_id(id)?;
    let mut transaction = repository::begin(pool).await?;
    repository::archive(&mut transaction, id).await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn select(pool: &SqlitePool, id: Option<&str>) -> Result<(), AppError> {
    if let Some(id) = id {
        domain::durable_id(id)?;
    }
    let mut transaction = repository::begin(pool).await?;
    repository::select(&mut transaction, id).await?;
    transaction.commit().await?;
    Ok(())
}

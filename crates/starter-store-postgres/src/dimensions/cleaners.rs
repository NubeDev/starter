//! Typed CRUD for the `cleaners` catalog.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;

use crate::pool::Pool;

/// Result alias.
pub type Result<T> = std::result::Result<T, sqlx::Error>;

/// `backfill` enum.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backfill {
    None,
    Sync,
    Async,
}

impl Backfill {
    pub fn as_str(&self) -> &'static str {
        match self {
            Backfill::None => "none",
            Backfill::Sync => "sync",
            Backfill::Async => "async",
        }
    }
}

/// `validate_entity` enum. `Strict` is rejected at define-time
/// because the dictionary lag bound (W11) makes it unimplementable
/// inside a pure-CH MV — the constraint is enforced by the
/// `cleaner.define` node, not by the database. `BestEffort` and
/// `None` are valid at-rest values.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidateEntity {
    Strict,
    BestEffort,
    None,
}

impl ValidateEntity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ValidateEntity::Strict => "strict",
            ValidateEntity::BestEffort => "best_effort",
            ValidateEntity::None => "none",
        }
    }
}

/// One row of `cleaners`.
#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct CleanerRow {
    pub name: String,
    pub description: Option<String>,
    pub source_table: String,
    pub target_table: String,
    pub filter: Json<serde_json::Value>,
    pub projection: Json<serde_json::Value>,
    pub definition_hash: String,
    pub backfill: String,
    pub validate_entity: String,
    pub mv_live_at: Option<DateTime<Utc>>,
    pub backfill_window_end: Option<DateTime<Utc>>,
    pub frozen_at_revision: Option<i64>,
    pub backfill_status: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub status: String,
}

/// Insert spec.
pub struct InsertCleaner<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub source_table: &'a str,
    pub target_table: &'a str,
    pub filter: &'a serde_json::Value,
    pub projection: &'a serde_json::Value,
    pub definition_hash: &'a str,
    pub backfill: Backfill,
    pub validate_entity: ValidateEntity,
    pub frozen_at_revision: Option<i64>,
    pub created_by: &'a str,
    pub status: &'a str,
}

/// Insert a cleaner row.
pub async fn insert(pool: &Pool, c: InsertCleaner<'_>) -> Result<CleanerRow> {
    sqlx::query_as::<_, CleanerRow>(
        "INSERT INTO cleaners \
            (name, description, source_table, target_table, filter, projection, \
             definition_hash, backfill, validate_entity, frozen_at_revision, \
             created_by, status) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
         RETURNING name, description, source_table, target_table, filter, projection, \
            definition_hash, backfill, validate_entity, mv_live_at, backfill_window_end, \
            frozen_at_revision, backfill_status, created_by, created_at, status",
    )
    .bind(c.name)
    .bind(c.description)
    .bind(c.source_table)
    .bind(c.target_table)
    .bind(Json(c.filter))
    .bind(Json(c.projection))
    .bind(c.definition_hash)
    .bind(c.backfill.as_str())
    .bind(c.validate_entity.as_str())
    .bind(c.frozen_at_revision)
    .bind(c.created_by)
    .bind(c.status)
    .fetch_one(pool.sqlx())
    .await
}

/// Mark MV live: set `mv_live_at = now()`, status = 'live', and
/// capture the backfill horizon.
pub async fn mark_live(
    pool: &Pool,
    name: &str,
    backfill_window_end: DateTime<Utc>,
) -> Result<u64> {
    let res = sqlx::query(
        "UPDATE cleaners SET \
            status = 'live', \
            mv_live_at = NOW(), \
            backfill_window_end = $2 \
         WHERE name = $1",
    )
    .bind(name)
    .bind(backfill_window_end)
    .execute(pool.sqlx())
    .await?;
    Ok(res.rows_affected())
}

/// Update the `backfill_status` field on the catalog row.
pub async fn set_backfill_status(pool: &Pool, name: &str, status: &str) -> Result<u64> {
    let res = sqlx::query("UPDATE cleaners SET backfill_status = $2 WHERE name = $1")
        .bind(name)
        .bind(status)
        .execute(pool.sqlx())
        .await?;
    Ok(res.rows_affected())
}

/// Transition status (drop / promote paths).
pub async fn set_status(pool: &Pool, name: &str, status: &str) -> Result<u64> {
    let res = sqlx::query("UPDATE cleaners SET status = $2 WHERE name = $1")
        .bind(name)
        .bind(status)
        .execute(pool.sqlx())
        .await?;
    Ok(res.rows_affected())
}

/// Fetch one cleaner by name.
pub async fn get(pool: &Pool, name: &str) -> Result<Option<CleanerRow>> {
    sqlx::query_as::<_, CleanerRow>(
        "SELECT name, description, source_table, target_table, filter, projection, \
            definition_hash, backfill, validate_entity, mv_live_at, backfill_window_end, \
            frozen_at_revision, backfill_status, created_by, created_at, status \
         FROM cleaners WHERE name = $1",
    )
    .bind(name)
    .fetch_optional(pool.sqlx())
    .await
}

/// Delete a row by name.
pub async fn delete(pool: &Pool, name: &str) -> Result<u64> {
    let res = sqlx::query("DELETE FROM cleaners WHERE name = $1")
        .bind(name)
        .execute(pool.sqlx())
        .await?;
    Ok(res.rows_affected())
}

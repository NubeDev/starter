//! Typed CRUD for the `sandboxes` catalog (RF-4).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;

use crate::pool::Pool;

/// Result alias.
pub type Result<T> = std::result::Result<T, sqlx::Error>;

/// One row of `sandboxes`.
#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct SandboxRow {
    pub name: String,
    pub description: Option<String>,
    pub owner: String,
    pub columns: Json<serde_json::Value>,
    pub columns_revision: i64,
    pub frozen_at_revision: Option<i64>,
    pub ttl_days: i32,
    pub promoted_to_cleaner: Option<String>,
    pub created_at: DateTime<Utc>,
    pub status: String,
}

/// Insert spec.
pub struct InsertSandbox<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub owner: &'a str,
    pub columns: &'a serde_json::Value,
    pub ttl_days: i32,
    pub status: &'a str,
}

/// Insert a sandbox row at `columns_revision = 1`.
pub async fn insert(pool: &Pool, s: InsertSandbox<'_>) -> Result<SandboxRow> {
    sqlx::query_as::<_, SandboxRow>(
        "INSERT INTO sandboxes \
            (name, description, owner, columns, ttl_days, status) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING name, description, owner, columns, columns_revision, \
            frozen_at_revision, ttl_days, promoted_to_cleaner, created_at, status",
    )
    .bind(s.name)
    .bind(s.description)
    .bind(s.owner)
    .bind(Json(s.columns))
    .bind(s.ttl_days)
    .bind(s.status)
    .fetch_one(pool.sqlx())
    .await
}

/// `sandbox.redefine` semantics: update the column set and bump
/// `columns_revision` atomically. Returns the new revision.
pub async fn redefine_columns(
    pool: &Pool,
    name: &str,
    columns: &serde_json::Value,
) -> Result<i64> {
    let (rev,): (i64,) = sqlx::query_as(
        "UPDATE sandboxes \
            SET columns = $2, columns_revision = columns_revision + 1 \
            WHERE name = $1 \
            RETURNING columns_revision",
    )
    .bind(name)
    .bind(Json(columns))
    .fetch_one(pool.sqlx())
    .await?;
    Ok(rev)
}

/// Pin a sandbox's `frozen_at_revision` (set by `cleaner.define`
/// when promoting from a sandbox). Drift between `columns_revision`
/// and `frozen_at_revision` after this point indicates the analyst
/// kept iterating on the sandbox after promotion.
pub async fn freeze(pool: &Pool, name: &str, promoted_to_cleaner: &str) -> Result<u64> {
    let res = sqlx::query(
        "UPDATE sandboxes SET \
            status = 'promoted', \
            promoted_to_cleaner = $2, \
            frozen_at_revision = columns_revision \
         WHERE name = $1",
    )
    .bind(name)
    .bind(promoted_to_cleaner)
    .execute(pool.sqlx())
    .await?;
    Ok(res.rows_affected())
}

/// Transition status.
pub async fn set_status(pool: &Pool, name: &str, status: &str) -> Result<u64> {
    let res = sqlx::query("UPDATE sandboxes SET status = $2 WHERE name = $1")
        .bind(name)
        .bind(status)
        .execute(pool.sqlx())
        .await?;
    Ok(res.rows_affected())
}

/// Fetch one row by name.
pub async fn get(pool: &Pool, name: &str) -> Result<Option<SandboxRow>> {
    sqlx::query_as::<_, SandboxRow>(
        "SELECT name, description, owner, columns, columns_revision, \
            frozen_at_revision, ttl_days, promoted_to_cleaner, created_at, status \
         FROM sandboxes WHERE name = $1",
    )
    .bind(name)
    .fetch_optional(pool.sqlx())
    .await
}

/// Delete a row by name.
pub async fn delete(pool: &Pool, name: &str) -> Result<u64> {
    let res = sqlx::query("DELETE FROM sandboxes WHERE name = $1")
        .bind(name)
        .execute(pool.sqlx())
        .await?;
    Ok(res.rows_affected())
}

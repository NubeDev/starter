//! Typed CRUD for the `marts` catalog (W5, W12).
//!
//! The DDL builder lives in `starter-warehouse`; this module owns
//! the Postgres seam — INSERT, status transitions, definition_hash
//! lookup, live-quota probe.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;

use crate::pool::Pool;

/// Lifecycle status of a mart catalog row.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MartStatus {
    Pending,
    Live,
    Quarantined,
    Failed,
}

impl MartStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MartStatus::Pending => "pending",
            MartStatus::Live => "live",
            MartStatus::Quarantined => "quarantined",
            MartStatus::Failed => "failed",
        }
    }
}

/// One row of `marts`. `time_bucket` is `PgInterval` (no serde
/// impl); serialise via `starter-warehouse` if a JSON envelope is
/// required.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct MartRow {
    pub name: String,
    pub description: Option<String>,
    pub source_table: String,
    pub filter: Json<serde_json::Value>,
    pub time_bucket: sqlx::postgres::types::PgInterval,
    pub group_by: Vec<String>,
    pub aggregations: Json<serde_json::Value>,
    pub definition_hash: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub status: String,
}

/// Insert spec. The catalog row lifecycle is owned by
/// `starter-warehouse`; this module accepts the desired initial
/// status verbatim (the W12 author-type table is enforced by the
/// caller).
pub struct InsertMart<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub source_table: &'a str,
    pub filter: &'a serde_json::Value,
    pub time_bucket: sqlx::postgres::types::PgInterval,
    pub group_by: &'a [String],
    pub aggregations: &'a serde_json::Value,
    pub definition_hash: &'a str,
    pub created_by: &'a str,
    pub status: MartStatus,
}

/// Result alias.
pub type Result<T> = std::result::Result<T, sqlx::Error>;

/// Insert a catalog row. The live-quota trigger fires here for
/// `status = 'live'` rows.
pub async fn insert(pool: &Pool, m: InsertMart<'_>) -> Result<MartRow> {
    sqlx::query_as::<_, MartRow>(
        "INSERT INTO marts \
            (name, description, source_table, filter, time_bucket, \
             group_by, aggregations, definition_hash, created_by, status) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         RETURNING name, description, source_table, filter, time_bucket, \
            group_by, aggregations, definition_hash, created_by, created_at, status",
    )
    .bind(m.name)
    .bind(m.description)
    .bind(m.source_table)
    .bind(Json(m.filter))
    .bind(m.time_bucket)
    .bind(m.group_by)
    .bind(Json(m.aggregations))
    .bind(m.definition_hash)
    .bind(m.created_by)
    .bind(m.status.as_str())
    .fetch_one(pool.sqlx())
    .await
}

/// Fetch one mart by name.
pub async fn get(pool: &Pool, name: &str) -> Result<Option<MartRow>> {
    sqlx::query_as::<_, MartRow>(
        "SELECT name, description, source_table, filter, time_bucket, \
            group_by, aggregations, definition_hash, created_by, created_at, status \
         FROM marts WHERE name = $1",
    )
    .bind(name)
    .fetch_optional(pool.sqlx())
    .await
}

/// Transition a mart to a new status. Live-quota trigger fires on
/// transitions into `live`.
pub async fn set_status(pool: &Pool, name: &str, status: MartStatus) -> Result<u64> {
    let res = sqlx::query("UPDATE marts SET status = $2 WHERE name = $1")
        .bind(name)
        .bind(status.as_str())
        .execute(pool.sqlx())
        .await?;
    Ok(res.rows_affected())
}

/// Count rows currently `status = 'live'`. The query uses the
/// `marts_live_count_idx` partial index, so this is O(live_count)
/// even with millions of `quarantined` rows in the table.
pub async fn live_count(pool: &Pool) -> Result<i64> {
    let (n,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM marts WHERE status = 'live'")
            .fetch_one(pool.sqlx())
            .await?;
    Ok(n)
}

/// Delete a row by name. Returns rows affected.
pub async fn delete(pool: &Pool, name: &str) -> Result<u64> {
    let res = sqlx::query("DELETE FROM marts WHERE name = $1")
        .bind(name)
        .execute(pool.sqlx())
        .await?;
    Ok(res.rows_affected())
}

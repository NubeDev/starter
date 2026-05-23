//! Typed CRUD for the `entities` dimension table.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;

use crate::pool::Pool;

/// One row of `entities`. The `tags` column is JSONB on the wire;
/// Rust callers see `serde_json::Value` (a tag set serialised via
/// `starter_tags::TagSet`).
#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct EntityRow {
    pub id: String,
    pub kind: String,
    pub display: Option<String>,
    pub tags: Json<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Result alias.
pub type Result<T> = std::result::Result<T, sqlx::Error>;

/// Insert a new entity. Fails on duplicate `id`.
pub async fn insert(
    pool: &Pool,
    id: &str,
    kind: &str,
    display: Option<&str>,
    tags: &serde_json::Value,
) -> Result<EntityRow> {
    sqlx::query_as::<_, EntityRow>(
        "INSERT INTO entities (id, kind, display, tags) \
         VALUES ($1, $2, $3, $4) \
         RETURNING id, kind, display, tags, created_at, updated_at",
    )
    .bind(id)
    .bind(kind)
    .bind(display)
    .bind(Json(tags))
    .fetch_one(pool.sqlx())
    .await
}

/// Upsert: insert or update by `id`. `updated_at` is bumped.
pub async fn upsert(
    pool: &Pool,
    id: &str,
    kind: &str,
    display: Option<&str>,
    tags: &serde_json::Value,
) -> Result<EntityRow> {
    sqlx::query_as::<_, EntityRow>(
        "INSERT INTO entities (id, kind, display, tags) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (id) DO UPDATE SET \
            kind = EXCLUDED.kind, \
            display = EXCLUDED.display, \
            tags = EXCLUDED.tags, \
            updated_at = NOW() \
         RETURNING id, kind, display, tags, created_at, updated_at",
    )
    .bind(id)
    .bind(kind)
    .bind(display)
    .bind(Json(tags))
    .fetch_one(pool.sqlx())
    .await
}

/// Fetch one entity by id.
pub async fn get(pool: &Pool, id: &str) -> Result<Option<EntityRow>> {
    sqlx::query_as::<_, EntityRow>(
        "SELECT id, kind, display, tags, created_at, updated_at \
         FROM entities WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool.sqlx())
    .await
}

/// Delete by id. Returns the number of rows removed (0 or 1).
pub async fn delete(pool: &Pool, id: &str) -> Result<u64> {
    let res = sqlx::query("DELETE FROM entities WHERE id = $1")
        .bind(id)
        .execute(pool.sqlx())
        .await?;
    Ok(res.rows_affected())
}

//! Typed CRUD for `entity_refs` (W6).

use serde::{Deserialize, Serialize};

use crate::pool::Pool;

/// One row of `entity_refs`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct EntityRefRow {
    pub from_id: String,
    pub rel: String,
    pub to_id: String,
}

/// Result alias.
pub type Result<T> = std::result::Result<T, sqlx::Error>;

/// Insert a ref. Idempotent: ON CONFLICT DO NOTHING on the composite PK.
pub async fn insert(pool: &Pool, from_id: &str, rel: &str, to_id: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO entity_refs (from_id, rel, to_id) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (from_id, rel, to_id) DO NOTHING",
    )
    .bind(from_id)
    .bind(rel)
    .bind(to_id)
    .execute(pool.sqlx())
    .await?;
    Ok(())
}

/// List outgoing refs from an entity.
pub async fn list_from(pool: &Pool, from_id: &str) -> Result<Vec<EntityRefRow>> {
    sqlx::query_as::<_, EntityRefRow>(
        "SELECT from_id, rel, to_id FROM entity_refs WHERE from_id = $1",
    )
    .bind(from_id)
    .fetch_all(pool.sqlx())
    .await
}

/// List incoming refs to an entity (uses the `entity_refs_to` index).
pub async fn list_to(pool: &Pool, to_id: &str) -> Result<Vec<EntityRefRow>> {
    sqlx::query_as::<_, EntityRefRow>(
        "SELECT from_id, rel, to_id FROM entity_refs WHERE to_id = $1",
    )
    .bind(to_id)
    .fetch_all(pool.sqlx())
    .await
}

/// Delete one ref. Returns rows removed.
pub async fn delete(pool: &Pool, from_id: &str, rel: &str, to_id: &str) -> Result<u64> {
    let res = sqlx::query(
        "DELETE FROM entity_refs WHERE from_id = $1 AND rel = $2 AND to_id = $3",
    )
    .bind(from_id)
    .bind(rel)
    .bind(to_id)
    .execute(pool.sqlx())
    .await?;
    Ok(res.rows_affected())
}

//! Catalog drift audit (W5).
//!
//! The mart catalog row carries a `definition_hash` over
//! `(filter, time_bucket, group_by, aggregations)`. If the row is
//! mutated outside `mart.define`, the recomputed hash diverges from
//! the stored one — that is the drift the W5 narrative is on guard
//! against. `find_hash_drift` returns the offending names so an
//! operator-side check can flag them; the orchestrator decides
//! whether to quarantine or hard-fail.

use serde::{Deserialize, Serialize};

use crate::pool::Pool;

/// One drift entry: the catalog name plus the hash recorded on the
/// row. Recomputation is the caller's job (it lives in
/// `starter-warehouse::ddl::mart`).
#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct CatalogHashRow {
    pub name: String,
    pub definition_hash: String,
    pub filter: sqlx::types::Json<serde_json::Value>,
    pub group_by: Vec<String>,
    pub aggregations: sqlx::types::Json<serde_json::Value>,
}

/// Result alias.
pub type Result<T> = std::result::Result<T, sqlx::Error>;

/// Stream every mart row for audit. The caller recomputes
/// `definition_hash` over the fetched fields and compares against
/// `row.definition_hash`; mismatches are drift.
pub async fn marts_for_audit(pool: &Pool) -> Result<Vec<CatalogHashRow>> {
    sqlx::query_as::<_, CatalogHashRow>(
        "SELECT name, definition_hash, filter, group_by, aggregations \
         FROM marts \
         ORDER BY name",
    )
    .fetch_all(pool.sqlx())
    .await
}

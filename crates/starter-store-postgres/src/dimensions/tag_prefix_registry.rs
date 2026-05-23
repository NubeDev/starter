//! Typed CRUD for `tag_prefix_registry` (Tags T6 BI-4).
//!
//! Two packs claiming the same prefix is a failure mode that must
//! surface at install time. The PRIMARY KEY on `prefix` is the
//! enforcement; this module wraps it with typed errors so the
//! pack installer can present a friendly message.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::pool::Pool;

/// One row of the prefix registry.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub struct PrefixRow {
    pub prefix: String,
    pub owner_pack: String,
    pub description: Option<String>,
    pub registered_at: DateTime<Utc>,
}

/// Why a prefix registration failed.
#[derive(Debug, thiserror::Error)]
pub enum RegisterError {
    /// Another pack already owns the prefix. The `existing_owner`
    /// field carries the conflicting `owner_pack`.
    #[error("prefix {prefix:?} is already owned by {existing_owner:?}")]
    Conflict {
        prefix: String,
        existing_owner: String,
    },
    /// Anything else (connection lost, CHECK constraint on the
    /// prefix shape, …).
    #[error(transparent)]
    Sql(#[from] sqlx::Error),
}

/// Register a prefix. Fails with [`RegisterError::Conflict`] when
/// the prefix already exists with a different owner.
pub async fn register(
    pool: &Pool,
    prefix: &str,
    owner_pack: &str,
    description: Option<&str>,
) -> Result<PrefixRow, RegisterError> {
    // The PK provides atomicity; we read the conflicting row back
    // only if the insert reports a unique violation.
    let res: Result<PrefixRow, sqlx::Error> = sqlx::query_as::<_, PrefixRow>(
        "INSERT INTO tag_prefix_registry (prefix, owner_pack, description) \
         VALUES ($1, $2, $3) \
         RETURNING prefix, owner_pack, description, registered_at",
    )
    .bind(prefix)
    .bind(owner_pack)
    .bind(description)
    .fetch_one(pool.sqlx())
    .await;
    match res {
        Ok(row) => Ok(row),
        Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
            let existing = lookup(pool, prefix)
                .await?
                .map(|r| r.owner_pack)
                .unwrap_or_default();
            Err(RegisterError::Conflict {
                prefix: prefix.to_string(),
                existing_owner: existing,
            })
        }
        Err(e) => Err(RegisterError::Sql(e)),
    }
}

/// Register inside a caller-supplied transaction. Returns
/// `Ok(None)` when the prefix conflicts so the caller can roll back
/// the entire pack install (Tags T6 BI-4: "two pack inserts claiming
/// the same prefix fail the txn"). Other errors propagate as `Err`.
pub async fn register_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    prefix: &str,
    owner_pack: &str,
    description: Option<&str>,
) -> Result<Option<PrefixRow>, sqlx::Error> {
    let res = sqlx::query_as::<_, PrefixRow>(
        "INSERT INTO tag_prefix_registry (prefix, owner_pack, description) \
         VALUES ($1, $2, $3) \
         RETURNING prefix, owner_pack, description, registered_at",
    )
    .bind(prefix)
    .bind(owner_pack)
    .bind(description)
    .fetch_one(&mut **tx)
    .await;
    match res {
        Ok(row) => Ok(Some(row)),
        Err(sqlx::Error::Database(db)) if db.is_unique_violation() => Ok(None),
        Err(e) => Err(e),
    }
}

/// Look up a row by prefix.
pub async fn lookup(pool: &Pool, prefix: &str) -> Result<Option<PrefixRow>, sqlx::Error> {
    sqlx::query_as::<_, PrefixRow>(
        "SELECT prefix, owner_pack, description, registered_at \
         FROM tag_prefix_registry WHERE prefix = $1",
    )
    .bind(prefix)
    .fetch_optional(pool.sqlx())
    .await
}

/// List every registered prefix.
pub async fn list(pool: &Pool) -> Result<Vec<PrefixRow>, sqlx::Error> {
    sqlx::query_as::<_, PrefixRow>(
        "SELECT prefix, owner_pack, description, registered_at \
         FROM tag_prefix_registry ORDER BY prefix",
    )
    .fetch_all(pool.sqlx())
    .await
}

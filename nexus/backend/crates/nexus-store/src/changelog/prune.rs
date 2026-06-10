//! Cross-tenant audit-ledger pruning — the system-actor delete behind retention.
//!
//! The retention sweep is a system actor, not a tenant request: it must delete
//! aged rows across every tenant, which RLS forbids the runtime role from doing
//! directly. This calls the SECURITY DEFINER `nexus_prune_changes` function
//! (migration `1603_changelog_retention.sql`) — the one controlled cross-tenant
//! write — rather than handing the runtime role BYPASSRLS. The function deletes
//! at most `batch` rows per call and returns the count, so the caller loops until
//! a batch comes back short.

use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use starter_spi::{Error, Result};

/// Delete up to `batch` ledger rows older than `cutoff`, across all tenants.
/// Returns the number deleted; a value below `batch` means the sweep is caught
/// up. The cap bounds how long the delete holds locks, so a large backlog drains
/// over several calls instead of one table-locking statement.
pub async fn prune_aged(pool: &PgPool, cutoff: DateTime<Utc>, batch: i32) -> Result<u64> {
    let row = sqlx::query("SELECT nexus_prune_changes($1, $2) AS deleted")
        .bind(cutoff)
        .bind(batch)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::Internal {
            source: Box::new(e),
        })?;
    let deleted: i64 = row.get("deleted");
    Ok(deleted.max(0) as u64)
}

//! Per-kind retention sweep for `starter_changes`.
//!
//! Reads `changelog_kind_policy` (added in migration
//! `0004_changelog_kind_policy.sql`) and deletes rows older than
//! each kind's `max_age_days`. Kinds with no row, or a row with
//! `max_age_days IS NULL`, are skipped — opting into a finite
//! retention curve is always explicit.
//!
//! See `rubix/docs/proposal/audit-log.md` for the rationale.

use starter_spi::{Error, Result};
use starter_store_postgres::Pool;

/// Per-kind row counts from a single [`apply_policy`] pass.
///
/// Empty when no kind has an opt-in policy. The caller decides
/// whether to log or expose; the helper itself is silent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PolicyReport {
    /// `(resource_kind, rows_deleted)` for each kind whose policy
    /// row specified a finite `max_age_days` at the time of the
    /// pass. Kinds with zero deletes are included so the caller
    /// can see "policy is in effect; nothing was over the line."
    pub per_kind: Vec<(String, u64)>,
}

impl PolicyReport {
    /// Sum of `rows_deleted` across every kind.
    pub fn total_deleted(&self) -> u64 {
        self.per_kind.iter().map(|(_, n)| n).sum()
    }
}

/// Apply the policy table once.
///
/// One `DELETE` per kind with an opt-in retention curve. Kinds with
/// no policy row or `NULL max_age_days` are left untouched —
/// matching the implicit-unbounded behaviour that has held since
/// the table was created.
///
/// Returns the per-kind row counts. Errors short-circuit; partial
/// progress from earlier kinds in the same call is not rolled back
/// (each kind is its own statement, deliberately, so a slow delete
/// on one kind cannot lock the whole audit table).
pub async fn apply_policy(pool: &Pool) -> Result<PolicyReport> {
    let opted_in: Vec<(String, i32)> = sqlx::query_as(
        "SELECT resource_kind, max_age_days \
         FROM changelog_kind_policy \
         WHERE max_age_days IS NOT NULL",
    )
    .fetch_all(pool.sqlx())
    .await
    .map_err(internal)?;

    let mut per_kind = Vec::with_capacity(opted_in.len());
    for (kind, days) in opted_in {
        let result = sqlx::query(
            "DELETE FROM starter_changes \
             WHERE resource_kind = $1 \
               AND at < NOW() - make_interval(days => $2)",
        )
        .bind(&kind)
        .bind(days)
        .execute(pool.sqlx())
        .await
        .map_err(internal)?;
        per_kind.push((kind, result.rows_affected()));
    }

    Ok(PolicyReport { per_kind })
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}

//! [`PgAuditPolicyStore`] \u{2014} Postgres-backed implementation of
//! [`rubix_spi::audit::AuditPolicyStore`] over the
//! `changelog_kind_policy` table.
//!
//! Table provenance: provisioned by the upstream `changelog`
//! migration source (see
//! `crates/starter-changelog-postgres/migrations/0004_changelog_kind_policy.sql`).
//! Audit-floor rows (`user`, `team`, `tenant`) are seeded by the
//! rubix-side [`crate::CHANGELOG_POLICY_MIGRATION_SOURCE`]; both
//! sources MUST be applied before this store is constructed.
//!
//! Contract per [`rubix_spi::audit::store`]:
//!
//! - [`list`](Self::list) \u{2014} stable order by
//!   `resource_kind ASC`. Backed by an `ORDER BY` clause rather
//!   than relying on table-scan order (Postgres makes no order
//!   guarantee without one).
//! - [`upsert`](Self::upsert) \u{2014} opens a transaction, takes
//!   `FOR UPDATE` on the prior row to serialise concurrent
//!   writers for the same kind, detects no-ops (same
//!   `max_age_days`) and commits the empty transaction without
//!   touching `updated_at`. Returns `(prior, new)` so the verb
//!   echo rule (\u{00A7}3.1) can rebuild the snapshot byte-exact.
//! - [`put`](Self::put) \u{2014} the undo path. Bypasses
//!   idempotency and restores the row verbatim, including its
//!   epoch-millisecond `updated_at_ms` (converted to TIMESTAMPTZ
//!   round-trip-safely via
//!   [`chrono::DateTime::from_timestamp_millis`]).
//! - [`delete`](Self::delete) \u{2014} hard delete; idempotent on
//!   missing rows (the trait contract is "the row no longer
//!   exists after this call", not "a row was actually deleted").
//!
//! Error mapping: every `sqlx::Error` is wrapped as
//! [`starter_spi::error::Error::Internal`] via the local
//! [`backend`] helper, mirroring the
//! [`crate::dashboards::PgDashboardStore`] pattern.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rubix_spi::audit::{AuditPolicyRow, AuditPolicyStore};
use rubix_spi::starter::error::{Error, Result};
use starter_store_postgres::pool::Pool;

/// Cheap-to-clone handle over the [`Pool`].
#[derive(Clone)]
pub struct PgAuditPolicyStore {
    pool: Pool,
}

impl PgAuditPolicyStore {
    /// Construct over an existing [`Pool`]. The upstream
    /// `changelog` migration source must have been applied first
    /// (see crate-level docs).
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

fn backend<E: std::error::Error + Send + Sync + 'static>(e: E) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}

/// Private decode shape. The wire type is
/// [`rubix_spi::audit::AuditPolicyRow`]; this struct exists only
/// to bridge the TIMESTAMPTZ column to the row's
/// `updated_at_ms: i64`.
#[derive(sqlx::FromRow)]
struct PolicyRow {
    resource_kind: String,
    max_age_days: Option<i32>,
    updated_at: DateTime<Utc>,
}

impl From<PolicyRow> for AuditPolicyRow {
    fn from(r: PolicyRow) -> Self {
        AuditPolicyRow {
            resource_kind: r.resource_kind,
            max_age_days: r.max_age_days,
            updated_at_ms: r.updated_at.timestamp_millis(),
        }
    }
}

const SELECT_COLS: &str = "resource_kind, max_age_days, updated_at";

#[async_trait]
impl AuditPolicyStore for PgAuditPolicyStore {
    async fn list(&self) -> Result<Vec<AuditPolicyRow>> {
        let sql = format!(
            "SELECT {SELECT_COLS}
               FROM changelog_kind_policy
              ORDER BY resource_kind ASC"
        );
        let rows: Vec<PolicyRow> = sqlx::query_as(&sql)
            .fetch_all(self.pool.sqlx())
            .await
            .map_err(backend)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get(&self, resource_kind: &str) -> Result<Option<AuditPolicyRow>> {
        let sql = format!(
            "SELECT {SELECT_COLS}
               FROM changelog_kind_policy
              WHERE resource_kind = $1
              LIMIT 1"
        );
        let row: Option<PolicyRow> = sqlx::query_as(&sql)
            .bind(resource_kind)
            .fetch_optional(self.pool.sqlx())
            .await
            .map_err(backend)?;
        Ok(row.map(Into::into))
    }

    async fn upsert(
        &self,
        resource_kind: &str,
        max_age_days: Option<i32>,
    ) -> Result<(Option<AuditPolicyRow>, AuditPolicyRow)> {
        // FOR UPDATE serialises concurrent same-key writers so
        // the no-op detection below cannot race with a peer that
        // flipped `max_age_days` between SELECT and UPDATE.
        let mut tx = self.pool.sqlx().begin().await.map_err(backend)?;

        let select_sql = format!(
            "SELECT {SELECT_COLS}
               FROM changelog_kind_policy
              WHERE resource_kind = $1
                FOR UPDATE"
        );
        let prior: Option<PolicyRow> = sqlx::query_as(&select_sql)
            .bind(resource_kind)
            .fetch_optional(&mut *tx)
            .await
            .map_err(backend)?;

        if let Some(p) = prior.as_ref() {
            if p.max_age_days == max_age_days {
                // No-op: same kind + same curve. Honour the
                // trait contract by returning identical rows
                // without touching `updated_at`. The empty
                // transaction still commits to release the lock.
                let row: AuditPolicyRow = AuditPolicyRow {
                    resource_kind: p.resource_kind.clone(),
                    max_age_days: p.max_age_days,
                    updated_at_ms: p.updated_at.timestamp_millis(),
                };
                tx.commit().await.map_err(backend)?;
                return Ok((Some(row.clone()), row));
            }
        }

        let upsert_sql = format!(
            "INSERT INTO changelog_kind_policy (resource_kind, max_age_days, updated_at)
                  VALUES ($1, $2, NOW())
             ON CONFLICT (resource_kind) DO UPDATE
                  SET max_age_days = EXCLUDED.max_age_days,
                      updated_at   = NOW()
              RETURNING {SELECT_COLS}"
        );
        let new_row: PolicyRow = sqlx::query_as(&upsert_sql)
            .bind(resource_kind)
            .bind(max_age_days)
            .fetch_one(&mut *tx)
            .await
            .map_err(backend)?;

        tx.commit().await.map_err(backend)?;
        Ok((prior.map(Into::into), new_row.into()))
    }

    async fn put(&self, row: AuditPolicyRow) -> Result<()> {
        // Round-trip the epoch-millisecond timestamp through
        // chrono so the column lands byte-exact (Postgres
        // TIMESTAMPTZ has microsecond resolution; ms always
        // fits). Reject out-of-range values up front rather than
        // letting sqlx silently truncate.
        let ts = DateTime::<Utc>::from_timestamp_millis(row.updated_at_ms).ok_or_else(|| {
            Error::Invalid {
                message: format!(
                    "PgAuditPolicyStore::put: AuditPolicyRow.updated_at_ms out of range: {}",
                    row.updated_at_ms
                ),
            }
        })?;

        sqlx::query(
            "INSERT INTO changelog_kind_policy (resource_kind, max_age_days, updated_at)
                  VALUES ($1, $2, $3)
             ON CONFLICT (resource_kind) DO UPDATE
                  SET max_age_days = EXCLUDED.max_age_days,
                      updated_at   = EXCLUDED.updated_at",
        )
        .bind(&row.resource_kind)
        .bind(row.max_age_days)
        .bind(ts)
        .execute(self.pool.sqlx())
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn delete(&self, resource_kind: &str) -> Result<()> {
        sqlx::query("DELETE FROM changelog_kind_policy WHERE resource_kind = $1")
            .bind(resource_kind)
            .execute(self.pool.sqlx())
            .await
            .map_err(backend)?;
        Ok(())
    }
}

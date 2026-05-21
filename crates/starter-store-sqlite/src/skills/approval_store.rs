//! [`SkillApprovalStore`] — SQLite-backed [`ApprovalStore`] impl.
//!
//! Schema is one table, `skill_approvals`, with the primary key
//! `(skill_id, hash)` that the registry's trust matrix keys off.
//! The trait is intentionally narrow (record / lookup / list /
//! revoke) so the SQL is one statement per call. Drift on registry
//! reload never reaches this store — see [`starter_skills::store`]
//! module docs for the append-mostly rule.
//!
//! All driver errors funnel through [`ApprovalStoreError::backend`]
//! so a future swap of the underlying driver does not leak into
//! callers.

use async_trait::async_trait;
use starter_flow_spi::skill::SkillId;
use starter_skills::{ApprovalRow, ApprovalStore, ApprovalStoreError};

use crate::pool::Pool;

/// SQLite-backed [`ApprovalStore`].
///
/// Clone is cheap — the underlying [`Pool`] is already arc'd
/// internally. Pair with [`super::SKILL_APPROVALS_MIGRATION_SOURCE`]
/// on engine boot.
#[derive(Clone)]
pub struct SkillApprovalStore {
    pool: Pool,
}

impl SkillApprovalStore {
    /// Construct a [`SkillApprovalStore`] over an existing [`Pool`].
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ApprovalStore for SkillApprovalStore {
    async fn record(&self, row: ApprovalRow) -> Result<(), ApprovalStoreError> {
        // Recording the same `(skill_id, hash)` twice is idempotent
        // per the trait contract; we let the second call refresh the
        // metadata (`approved_at` / `approved_by`) so an operator
        // re-approval bumps the audit trail without producing a
        // separate row (which the PK forbids anyway).
        sqlx::query(
            "INSERT INTO skill_approvals \
                 (skill_id, hash, approved_at, approved_by) \
                 VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT (skill_id, hash) DO UPDATE SET \
                 approved_at = excluded.approved_at, \
                 approved_by = excluded.approved_by",
        )
        .bind(row.skill_id.as_str())
        .bind(&row.bundle_hash)
        // SQLite has no u64; widen to i64. Unix-ms values stay
        // positive for ~292 million years, so the cast is safe.
        .bind(row.approved_at_unix_ms as i64)
        .bind(&row.approved_by)
        .execute(self.pool.sqlx())
        .await
        .map_err(ApprovalStoreError::backend)?;
        Ok(())
    }

    async fn lookup(
        &self,
        skill_id: &SkillId,
        bundle_hash: &str,
    ) -> Result<Option<ApprovalRow>, ApprovalStoreError> {
        let row: Option<(String, String, i64, String)> = sqlx::query_as(
            "SELECT skill_id, hash, approved_at, approved_by \
                 FROM skill_approvals \
                 WHERE skill_id = ?1 AND hash = ?2",
        )
        .bind(skill_id.as_str())
        .bind(bundle_hash)
        .fetch_optional(self.pool.sqlx())
        .await
        .map_err(ApprovalStoreError::backend)?;

        row.map(row_to_approval).transpose()
    }

    async fn list(&self) -> Result<Vec<ApprovalRow>, ApprovalStoreError> {
        let rows: Vec<(String, String, i64, String)> = sqlx::query_as(
            "SELECT skill_id, hash, approved_at, approved_by \
                 FROM skill_approvals",
        )
        .fetch_all(self.pool.sqlx())
        .await
        .map_err(ApprovalStoreError::backend)?;

        rows.into_iter().map(row_to_approval).collect()
    }

    async fn revoke(
        &self,
        skill_id: &SkillId,
        bundle_hash: &str,
    ) -> Result<(), ApprovalStoreError> {
        // DELETE of a missing row is a no-op in SQL — matches the
        // trait contract without any pre-check round-trip.
        sqlx::query("DELETE FROM skill_approvals WHERE skill_id = ?1 AND hash = ?2")
            .bind(skill_id.as_str())
            .bind(bundle_hash)
            .execute(self.pool.sqlx())
            .await
            .map_err(ApprovalStoreError::backend)?;
        Ok(())
    }
}

/// Map a row tuple to [`ApprovalRow`]. `SkillId::new` validates the
/// stored id — a NULL/garbage row surfaces as a typed backend
/// error, not a panic.
fn row_to_approval(
    (skill_id, hash, approved_at, approved_by): (String, String, i64, String),
) -> Result<ApprovalRow, ApprovalStoreError> {
    let skill_id = SkillId::new(skill_id).map_err(ApprovalStoreError::backend)?;
    Ok(ApprovalRow {
        skill_id,
        bundle_hash: hash,
        approved_by,
        // approved_at is always written as a non-negative i64; if a
        // future operator tool stuffs a negative value in by hand,
        // clamp to 0 rather than wrap.
        approved_at_unix_ms: approved_at.max(0) as u64,
    })
}

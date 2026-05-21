//! [`SkillApprovalStore`] — Postgres-backed [`ApprovalStore`] impl.
//!
//! Twin of `starter_store_sqlite::skills::SkillApprovalStore`; the
//! only differences are placeholder syntax (`$N` vs `?N`),
//! `ON CONFLICT` upsert spelling (identical), and the
//! `approved_at` column type — Postgres uses `TIMESTAMPTZ`, so we
//! marshal the trait's `u64` Unix-ms to/from epoch seconds inline
//! to avoid pulling `chrono`/`time` into this crate's dep tree.

use async_trait::async_trait;
use starter_flow_spi::skill::SkillId;
use starter_skills::{ApprovalRow, ApprovalStore, ApprovalStoreError};

use crate::pool::Pool;

/// Postgres-backed [`ApprovalStore`].
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
        // Bind millis as `double precision` seconds and let
        // `to_timestamp` build the TIMESTAMPTZ — no chrono dep.
        let approved_at_seconds = (row.approved_at_unix_ms as f64) / 1000.0;
        sqlx::query(
            "INSERT INTO skill_approvals \
                 (skill_id, hash, approved_at, approved_by) \
                 VALUES ($1, $2, to_timestamp($3), $4) \
             ON CONFLICT (skill_id, hash) DO UPDATE SET \
                 approved_at = excluded.approved_at, \
                 approved_by = excluded.approved_by",
        )
        .bind(row.skill_id.as_str())
        .bind(&row.bundle_hash)
        .bind(approved_at_seconds)
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
        // `(EXTRACT(EPOCH FROM ts) * 1000)::BIGINT` round-trips
        // millis losslessly for any value SQLite/Postgres would
        // store via `to_timestamp(seconds)` above.
        let row: Option<(String, String, i64, String)> = sqlx::query_as(
            "SELECT skill_id, \
                    hash, \
                    (EXTRACT(EPOCH FROM approved_at) * 1000)::BIGINT, \
                    approved_by \
                 FROM skill_approvals \
                 WHERE skill_id = $1 AND hash = $2",
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
            "SELECT skill_id, \
                    hash, \
                    (EXTRACT(EPOCH FROM approved_at) * 1000)::BIGINT, \
                    approved_by \
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
        sqlx::query("DELETE FROM skill_approvals WHERE skill_id = $1 AND hash = $2")
            .bind(skill_id.as_str())
            .bind(bundle_hash)
            .execute(self.pool.sqlx())
            .await
            .map_err(ApprovalStoreError::backend)?;
        Ok(())
    }
}

fn row_to_approval(
    (skill_id, hash, approved_at_ms, approved_by): (String, String, i64, String),
) -> Result<ApprovalRow, ApprovalStoreError> {
    let skill_id = SkillId::new(skill_id).map_err(ApprovalStoreError::backend)?;
    Ok(ApprovalRow {
        skill_id,
        bundle_hash: hash,
        approved_by,
        approved_at_unix_ms: approved_at_ms.max(0) as u64,
    })
}

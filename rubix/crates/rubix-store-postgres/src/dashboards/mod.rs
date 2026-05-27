//! [`PgDashboardStore`] — Postgres-backed implementation of
//! [`rubix_spi::dashboard::DashboardStore`] over the
//! `dashboards_definitions` table.
//!
//! Phase A.1 contract (per
//! `rubix/docs/scope/dashboards/01-storage.md`):
//!
//! - **Insert-only writes.** [`insert_revision`](Self::insert_revision)
//!   opens a transaction, supersedes any live row for
//!   `(tenant_id, page_id)`, inserts the new revision, and commits;
//!   the active partial index never sees two heads.
//! - **Active reads.** [`get_active`](Self::get_active) +
//!   [`list_active`](Self::list_active) filter on
//!   `superseded_at IS NULL`.
//! - **Soft-delete.** [`mark_superseded`](Self::mark_superseded) is
//!   the only path that supersedes without inserting a replacement
//!   — wired into the future `rubix.dashboard.delete` tool body.
//! - **History.** [`history`](Self::history) returns every revision
//!   in `created_at DESC` order; powers the audit UI.

use async_trait::async_trait;
use rubix_spi::dashboard::{
    DashboardRevision, DashboardStore, DashboardStoreError, InsertOutcome, ListFilter, NewRevision,
};
use starter_store_postgres::pool::Pool;
use uuid::Uuid;

/// Cheap-to-clone handle over the [`Pool`].
#[derive(Clone)]
pub struct PgDashboardStore {
    pool: Pool,
}

impl PgDashboardStore {
    /// Construct over an existing [`Pool`]. The
    /// `dashboards_definitions` migration source must have been
    /// applied first (see
    /// [`crate::DASHBOARDS_DEFINITIONS_MIGRATION_SOURCE`]).
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

fn backend<E: std::fmt::Display>(e: E) -> DashboardStoreError {
    DashboardStoreError::Backend(e.to_string())
}

/// Shape every `SELECT` in this module decodes into. Kept private
/// so the wire type stays
/// [`rubix_spi::dashboard::DashboardRevision`].
#[derive(sqlx::FromRow)]
struct Row {
    page_id: String,
    revision_id: Uuid,
    tenant_id: String,
    owner_principal: String,
    title: String,
    tags: Vec<String>,
    body_json: serde_json::Value,
    created_by: String,
    created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
    superseded_at: Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>>,
}

impl From<Row> for DashboardRevision {
    fn from(r: Row) -> Self {
        DashboardRevision {
            page_id: r.page_id,
            revision_id: r.revision_id.to_string(),
            tenant_id: r.tenant_id,
            owner_principal: r.owner_principal,
            title: r.title,
            tags: r.tags,
            body_json: r.body_json,
            created_by: r.created_by,
            created_at: r.created_at.to_rfc3339(),
            superseded_at: r.superseded_at.map(|t| t.to_rfc3339()),
        }
    }
}

const SELECT_COLS: &str = "page_id, revision_id, tenant_id, owner_principal, title, tags, \
     body_json, created_by, created_at, superseded_at";

#[async_trait]
impl DashboardStore for PgDashboardStore {
    async fn insert_revision(
        &self,
        new_revision: NewRevision,
    ) -> Result<DashboardRevision, DashboardStoreError> {
        let mut tx = self.pool.sqlx().begin().await.map_err(backend)?;

        // Supersede any prior live row for this (tenant, page_id).
        // The single-active invariant is enforced here, not by a
        // unique partial index, because the writer is the sole
        // path that mutates the column.
        sqlx::query(
            "UPDATE dashboards_definitions
                SET superseded_at = NOW()
              WHERE tenant_id = $1
                AND page_id = $2
                AND superseded_at IS NULL",
        )
        .bind(&new_revision.tenant_id)
        .bind(&new_revision.page_id)
        .execute(&mut *tx)
        .await
        .map_err(backend)?;

        let revision_id = Uuid::new_v4();
        let sql = format!(
            "INSERT INTO dashboards_definitions
                (page_id, revision_id, body_json, tenant_id, owner_principal,
                 title, tags, created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING {SELECT_COLS}"
        );
        let row: Row = sqlx::query_as(&sql)
            .bind(&new_revision.page_id)
            .bind(revision_id)
            .bind(&new_revision.body_json)
            .bind(&new_revision.tenant_id)
            .bind(&new_revision.owner_principal)
            .bind(&new_revision.title)
            .bind(&new_revision.tags)
            .bind(&new_revision.created_by)
            .fetch_one(&mut *tx)
            .await
            .map_err(backend)?;

        tx.commit().await.map_err(backend)?;
        Ok(row.into())
    }

    /// Atomic variant. Captures the prior row via
    /// `UPDATE ... RETURNING` in the same transaction as the
    /// supersede + insert, so the audit recorder sees the
    /// before-state without any TOCTOU window. Falls back to the
    /// default impl's two-step pattern only when no row was
    /// superseded (in which case `prior` is `None` and no race is
    /// possible by definition).
    async fn insert_revision_with_prior(
        &self,
        new_revision: NewRevision,
    ) -> Result<InsertOutcome, DashboardStoreError> {
        let mut tx = self.pool.sqlx().begin().await.map_err(backend)?;

        let supersede_sql = format!(
            "UPDATE dashboards_definitions
                SET superseded_at = NOW()
              WHERE tenant_id = $1
                AND page_id = $2
                AND superseded_at IS NULL
            RETURNING {SELECT_COLS}"
        );
        let prior_row: Option<Row> = sqlx::query_as(&supersede_sql)
            .bind(&new_revision.tenant_id)
            .bind(&new_revision.page_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(backend)?;

        let revision_id = Uuid::new_v4();
        let insert_sql = format!(
            "INSERT INTO dashboards_definitions
                (page_id, revision_id, body_json, tenant_id, owner_principal,
                 title, tags, created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING {SELECT_COLS}"
        );
        let inserted_row: Row = sqlx::query_as(&insert_sql)
            .bind(&new_revision.page_id)
            .bind(revision_id)
            .bind(&new_revision.body_json)
            .bind(&new_revision.tenant_id)
            .bind(&new_revision.owner_principal)
            .bind(&new_revision.title)
            .bind(&new_revision.tags)
            .bind(&new_revision.created_by)
            .fetch_one(&mut *tx)
            .await
            .map_err(backend)?;

        tx.commit().await.map_err(backend)?;
        Ok(InsertOutcome {
            inserted: inserted_row.into(),
            prior: prior_row.map(Into::into),
        })
    }

    async fn get_active(
        &self,
        tenant_id: &str,
        page_id: &str,
    ) -> Result<Option<DashboardRevision>, DashboardStoreError> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM dashboards_definitions
              WHERE tenant_id = $1 AND page_id = $2 AND superseded_at IS NULL
              LIMIT 1"
        );
        let row: Option<Row> = sqlx::query_as(&sql)
            .bind(tenant_id)
            .bind(page_id)
            .fetch_optional(self.pool.sqlx())
            .await
            .map_err(backend)?;
        Ok(row.map(Into::into))
    }

    async fn list_active(
        &self,
        tenant_id: &str,
        filter: &ListFilter,
    ) -> Result<Vec<DashboardRevision>, DashboardStoreError> {
        // Use parameterised `$3::text[] = '{}'` short-circuit so we
        // can stay on one prepared statement regardless of whether
        // the caller passed `tags_any`.
        let sql = format!(
            "SELECT {SELECT_COLS} FROM dashboards_definitions
              WHERE tenant_id = $1
                AND superseded_at IS NULL
                AND ($2::text IS NULL OR owner_principal = $2)
                AND (cardinality($3::text[]) = 0 OR tags && $3::text[])
              ORDER BY created_at DESC"
        );
        let rows: Vec<Row> = sqlx::query_as(&sql)
            .bind(tenant_id)
            .bind(filter.owner.as_deref())
            .bind(&filter.tags_any)
            .fetch_all(self.pool.sqlx())
            .await
            .map_err(backend)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn mark_superseded(
        &self,
        tenant_id: &str,
        page_id: &str,
    ) -> Result<u64, DashboardStoreError> {
        let res = sqlx::query(
            "UPDATE dashboards_definitions
                SET superseded_at = NOW()
              WHERE tenant_id = $1
                AND page_id = $2
                AND superseded_at IS NULL",
        )
        .bind(tenant_id)
        .bind(page_id)
        .execute(self.pool.sqlx())
        .await
        .map_err(backend)?;
        Ok(res.rows_affected())
    }

    async fn history(&self, page_id: &str) -> Result<Vec<DashboardRevision>, DashboardStoreError> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM dashboards_definitions
              WHERE page_id = $1
              ORDER BY created_at DESC"
        );
        let rows: Vec<Row> = sqlx::query_as(&sql)
            .bind(page_id)
            .fetch_all(self.pool.sqlx())
            .await
            .map_err(backend)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

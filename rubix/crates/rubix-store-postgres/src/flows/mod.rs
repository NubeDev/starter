//! [`PgFlowDefStore`] — Postgres-backed implementation of
//! [`rubix_spi::flow_def::FlowDefStore`] over the
//! `flows_definitions` table.
//!
//! Mirrors [`crate::dashboards::PgDashboardStore`] in shape and
//! contract:
//!
//! - **Insert-only writes.** [`insert_revision`](FlowDefStore::insert_revision)
//!   opens a transaction, supersedes any live row for `flow_id`,
//!   inserts the new revision, and commits. The single-active
//!   invariant (at most one row per `flow_id` with
//!   `superseded_at IS NULL`) is enforced by the writer rather
//!   than by a unique partial index because the writer is the
//!   sole path that mutates `superseded_at`.
//! - **Active reads.** [`fetch_latest_live`](FlowDefStore::fetch_latest_live)
//!   and [`list_live`](FlowDefStore::list_live) both filter on
//!   `superseded_at IS NULL`.
//! - **Soft delete + undo.** [`mark_superseded`](FlowDefStore::mark_superseded)
//!   and [`clear_superseded`](FlowDefStore::clear_superseded)
//!   are the undo dispatcher's walk-forward / walk-backward
//!   primitives (see [`rubix-tools::flow_ops::store::FlowDefReversible`]).
//!
//! Tenant scoping: the rubix surface today writes every revision
//! under [`SYSTEM_TENANT`] (the all-zero UUID also used by
//! [`rubix-agent::boot::flows_seed::SYSTEM_TENANT`]). Multi-tenant
//! routing lands when the `flow_ops` verbs grow a tenant
//! parameter; until then both columns are pinned here so the row
//! shape matches the bundled-seed rows and the
//! `flows_definitions_unique_revision` constraint is honoured.

use async_trait::async_trait;
use rubix_spi::flow_def::{FlowDefStore, FlowRevisionRow};
use rubix_spi::starter::error::{Error, Result};
use starter_store_postgres::pool::Pool;
use uuid::Uuid;

/// The all-zero UUID used as both `tenant_id` and `created_by`
/// for rows the rubix surface writes today. Mirrors
/// [`rubix-agent::boot::flows_seed::SYSTEM_TENANT`].
pub const SYSTEM_TENANT: Uuid = Uuid::nil();

/// Cheap-to-clone handle over the [`Pool`].
#[derive(Clone)]
pub struct PgFlowDefStore {
    pool: Pool,
}

impl PgFlowDefStore {
    /// Construct over an existing [`Pool`]. The
    /// `flows_definitions` migration source must have been
    /// applied first (see
    /// [`crate::FLOWS_DEFINITIONS_MIGRATION_SOURCE`]).
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

fn backend<E: std::fmt::Display>(e: E) -> Error {
    Error::Internal {
        source: Box::new(std::io::Error::other(format!("flows_definitions: {e}"))),
    }
}

/// Shape every `SELECT` in this module decodes into.
#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    flow_id: String,
    revision_id: String,
    body_yaml: String,
    created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
    superseded_at: Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>>,
}

impl From<Row> for FlowRevisionRow {
    fn from(r: Row) -> Self {
        FlowRevisionRow {
            id: r.id,
            flow_id: r.flow_id,
            revision_id: r.revision_id,
            body_yaml: r.body_yaml,
            created_at_ms: r.created_at.timestamp_millis(),
            superseded_at_ms: r.superseded_at.map(|t| t.timestamp_millis()),
        }
    }
}

const SELECT_COLS: &str = "id, flow_id, revision_id, body_yaml, created_at, superseded_at";

/// Generate a ULID-shaped text id for the `id` primary key.
/// Sized to fit `TEXT` without committing to ULID library deps;
/// the column accepts arbitrary text and the seeder uses the
/// same hex-uuid shape (`flows_seed.rs::ulid_text`).
fn fresh_id() -> String {
    Uuid::new_v4().simple().to_string()
}

#[async_trait]
impl FlowDefStore for PgFlowDefStore {
    async fn insert_revision(
        &self,
        flow_id: &str,
        body_yaml: &str,
        _now_ms: i64,
    ) -> Result<(FlowRevisionRow, Option<String>)> {
        // `_now_ms` is ignored: the table's `created_at` column
        // defaults to `NOW()` and `superseded_at` is stamped by
        // the writer with `NOW()` as well so wall-clock skew
        // between the caller and the database is never load-
        // bearing for the single-active invariant.
        let mut tx = self.pool.sqlx().begin().await.map_err(backend)?;

        // Supersede the prior live row, if any, and capture its
        // revision_id so the verb response (and the Reversible
        // snapshot) can carry it. Locks the row `FOR UPDATE`
        // inside the same tx so a concurrent deploy can't slip
        // two live revisions past the invariant.
        let prior_revision_id: Option<String> = sqlx::query_scalar(
            "UPDATE flows_definitions
                SET superseded_at = NOW()
              WHERE tenant_id = $1::uuid
                AND flow_id = $2
                AND superseded_at IS NULL
              RETURNING revision_id",
        )
        .bind(SYSTEM_TENANT)
        .bind(flow_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(backend)?;

        let id = fresh_id();
        let revision_id = Uuid::new_v4().to_string();
        let sql = format!(
            "INSERT INTO flows_definitions
                (id, tenant_id, flow_id, revision_id, body_yaml, created_by)
             VALUES ($1, $2::uuid, $3, $4, $5, $6::uuid)
             RETURNING {SELECT_COLS}"
        );
        let row: Row = sqlx::query_as(&sql)
            .bind(&id)
            .bind(SYSTEM_TENANT)
            .bind(flow_id)
            .bind(&revision_id)
            .bind(body_yaml)
            .bind(SYSTEM_TENANT)
            .fetch_one(&mut *tx)
            .await
            .map_err(backend)?;

        tx.commit().await.map_err(backend)?;
        Ok((row.into(), prior_revision_id))
    }

    async fn fetch_latest_live(&self, flow_id: &str) -> Result<Option<FlowRevisionRow>> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM flows_definitions
              WHERE tenant_id = $1::uuid
                AND flow_id = $2
                AND superseded_at IS NULL
              LIMIT 1"
        );
        let row: Option<Row> = sqlx::query_as(&sql)
            .bind(SYSTEM_TENANT)
            .bind(flow_id)
            .fetch_optional(self.pool.sqlx())
            .await
            .map_err(backend)?;
        Ok(row.map(Into::into))
    }

    async fn list_live(&self) -> Result<Vec<FlowRevisionRow>> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM flows_definitions
              WHERE tenant_id = $1::uuid AND superseded_at IS NULL
              ORDER BY flow_id ASC"
        );
        let rows: Vec<Row> = sqlx::query_as(&sql)
            .bind(SYSTEM_TENANT)
            .fetch_all(self.pool.sqlx())
            .await
            .map_err(backend)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn mark_superseded(&self, revision_id: &str, _now_ms: i64) -> Result<()> {
        let res = sqlx::query(
            "UPDATE flows_definitions
                SET superseded_at = NOW()
              WHERE revision_id = $1
                AND superseded_at IS NULL",
        )
        .bind(revision_id)
        .execute(self.pool.sqlx())
        .await
        .map_err(backend)?;
        if res.rows_affected() == 0 {
            return Err(Error::NotFound {
                what: format!("flow_definition revision:{revision_id}"),
            });
        }
        Ok(())
    }

    async fn clear_superseded(&self, revision_id: &str) -> Result<()> {
        let res = sqlx::query(
            "UPDATE flows_definitions
                SET superseded_at = NULL
              WHERE revision_id = $1",
        )
        .bind(revision_id)
        .execute(self.pool.sqlx())
        .await
        .map_err(backend)?;
        if res.rows_affected() == 0 {
            return Err(Error::NotFound {
                what: format!("flow_definition revision:{revision_id}"),
            });
        }
        Ok(())
    }
}

//! [`SqliteFlowStore`] — flow definitions, immutable revisions,
//! per-flow head pointer (D-F3.3).

use async_trait::async_trait;
use starter_flow_spi::flow::{
    FlowError, FlowId, FlowResult, FlowRevision, FlowRevisionId, FlowStore,
};

use super::schema::{from_json, sqlx_backend, sqlx_to_flow, to_json};
use crate::pool::Pool;

/// SQLite-backed [`FlowStore`].
///
/// Cheap to clone — wraps the ref-counted [`Pool`].
#[derive(Clone)]
pub struct SqliteFlowStore {
    pool: Pool,
}

impl SqliteFlowStore {
    /// Construct a [`SqliteFlowStore`] over an existing [`Pool`].
    /// The flow migrations must have been applied first via the
    /// [`super::FLOW_MIGRATION_SOURCE`] entry.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FlowStore for SqliteFlowStore {
    async fn load(
        &self,
        flow_id: FlowId,
        revision: Option<FlowRevisionId>,
    ) -> FlowResult<FlowRevision> {
        let pool = self.pool.sqlx();
        let flow_id_s = flow_id.as_str().to_string();

        // Resolve the target revision id: explicit, or the flow's
        // current head. A flow with no head is `NotFound`.
        let revision_id = match revision {
            Some(r) => r,
            None => {
                let row: Option<(String,)> =
                    sqlx::query_as("SELECT revision_id FROM flow_heads WHERE flow_id = ?1")
                        .bind(&flow_id_s)
                        .fetch_optional(pool)
                        .await
                        .map_err(sqlx_backend)?;
                let raw = row
                    .ok_or_else(|| FlowError::NotFound {
                        kind: "flow",
                        id: flow_id_s.clone(),
                    })?
                    .0;
                FlowRevisionId(
                    uuid::Uuid::parse_str(&raw)
                        .map_err(|e| FlowError::Backend(format!("flow_heads.revision_id: {e}")))?,
                )
            }
        };

        let row: (String, String) = sqlx::query_as(
            "SELECT body_json, source FROM flow_revisions \
             WHERE flow_id = ?1 AND revision_id = ?2",
        )
        .bind(&flow_id_s)
        .bind(revision_id.0.to_string())
        .fetch_one(pool)
        .await
        .map_err(|e| sqlx_to_flow(e, "flow_revision", revision_id.0.to_string()))?;

        let body = from_json::<serde_json::Value>("flow_revisions.body_json", &row.0)?;
        Ok(FlowRevision::new(flow_id, revision_id, body).with_source(row.1))
    }

    async fn put(&self, revision: FlowRevision) -> FlowResult<FlowRevisionId> {
        let pool = self.pool.sqlx();
        let body_json = to_json(&revision.body)?;
        let flow_id_s = revision.flow_id.as_str().to_string();
        let revision_id_s = revision.revision_id.0.to_string();

        let mut tx = pool.begin().await.map_err(sqlx_backend)?;

        // INSERT OR IGNORE so re-puts of the same (flow_id,
        // revision_id) pair are idempotent — revisions are
        // immutable per SCOPE "Decisions made"; a body mismatch
        // would be a caller bug we'd rather not silently overwrite,
        // but detecting it costs an extra round-trip the store
        // contract doesn't currently mandate. Stage 5 wires the
        // engine-side guard.
        sqlx::query(
            "INSERT OR IGNORE INTO flow_revisions \
             (flow_id, revision_id, body_json, source) VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(&flow_id_s)
        .bind(&revision_id_s)
        .bind(&body_json)
        .bind(&revision.source)
        .execute(&mut *tx)
        .await
        .map_err(sqlx_backend)?;

        // Advance the head pointer; later `load(.., None)`
        // resolves to this revision until a future `put` advances
        // it again.
        sqlx::query(
            "INSERT INTO flow_heads (flow_id, revision_id, updated_at) \
             VALUES (?1, ?2, CURRENT_TIMESTAMP) \
             ON CONFLICT (flow_id) DO UPDATE SET \
                 revision_id = excluded.revision_id, \
                 updated_at = CURRENT_TIMESTAMP",
        )
        .bind(&flow_id_s)
        .bind(&revision_id_s)
        .execute(&mut *tx)
        .await
        .map_err(sqlx_backend)?;

        tx.commit().await.map_err(sqlx_backend)?;
        Ok(revision.revision_id)
    }

    async fn list(&self) -> FlowResult<Vec<FlowId>> {
        let pool = self.pool.sqlx();
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT DISTINCT flow_id FROM flow_revisions ORDER BY flow_id")
                .fetch_all(pool)
                .await
                .map_err(sqlx_backend)?;
        rows.into_iter()
            .map(|(s,)| FlowId::new(s).map_err(|e| FlowError::Backend(format!("flow_id: {e}"))))
            .collect()
    }

    async fn revisions(&self, flow_id: FlowId) -> FlowResult<Vec<FlowRevisionId>> {
        let pool = self.pool.sqlx();
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT revision_id FROM flow_revisions \
             WHERE flow_id = ?1 \
             ORDER BY created_at DESC, revision_id DESC",
        )
        .bind(flow_id.as_str())
        .fetch_all(pool)
        .await
        .map_err(sqlx_backend)?;
        rows.into_iter()
            .map(|(s,)| {
                uuid::Uuid::parse_str(&s)
                    .map(FlowRevisionId)
                    .map_err(|e| FlowError::Backend(format!("flow_revisions.revision_id: {e}")))
            })
            .collect()
    }

    async fn head(&self, flow_id: FlowId) -> FlowResult<Option<FlowRevisionId>> {
        let pool = self.pool.sqlx();
        let row: Option<(String,)> =
            sqlx::query_as("SELECT revision_id FROM flow_heads WHERE flow_id = ?1")
                .bind(flow_id.as_str())
                .fetch_optional(pool)
                .await
                .map_err(sqlx_backend)?;
        row.map(|(s,)| {
            uuid::Uuid::parse_str(&s)
                .map(FlowRevisionId)
                .map_err(|e| FlowError::Backend(format!("flow_heads.revision_id: {e}")))
        })
        .transpose()
    }
}

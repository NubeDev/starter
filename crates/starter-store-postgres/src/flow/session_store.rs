//! [`PgSessionStore`] — session records and principal-scoped
//! listing (D-F3.3). Postgres twin of `SqliteSessionStore`.

use async_trait::async_trait;
use starter_flow_spi::flow::{FlowError, FlowResult, SessionId, SessionRecord, SessionStore};
use starter_flow_spi::Principal;

use super::schema::{from_value, sqlx_backend, to_value};
use crate::pool::Pool;

/// Postgres-backed [`SessionStore`].
#[derive(Clone)]
pub struct PgSessionStore {
    pool: Pool,
}

impl PgSessionStore {
    /// Construct a [`PgSessionStore`] over an existing [`Pool`].
    /// Pair with [`super::FLOW_MIGRATION_SOURCE`].
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionStore for PgSessionStore {
    async fn get(&self, session_id: SessionId) -> FlowResult<Option<SessionRecord>> {
        let pool = self.pool.sqlx();
        let row: Option<(serde_json::Value, serde_json::Value)> =
            sqlx::query_as("SELECT principal_json, body_json FROM sessions WHERE session_id = $1")
                .bind(session_id.0.to_string())
                .fetch_optional(pool)
                .await
                .map_err(sqlx_backend)?;
        let Some((principal_value, body_value)) = row else {
            return Ok(None);
        };
        let principal: Principal = from_value("sessions.principal_json", principal_value)?;
        let body: serde_json::Value = from_value("sessions.body_json", body_value)?;
        Ok(Some(SessionRecord::new(session_id, principal, body)))
    }

    async fn put(&self, session_id: SessionId, record: SessionRecord) -> FlowResult<()> {
        let pool = self.pool.sqlx();
        let principal_value = to_value(&record.principal)?;
        let body_value = to_value(&record.body)?;
        sqlx::query(
            "INSERT INTO sessions \
                 (session_id, principal_json, body_json, updated_at) \
                 VALUES ($1, $2, $3, NOW()) \
             ON CONFLICT (session_id) DO UPDATE SET \
                 principal_json = excluded.principal_json, \
                 body_json      = excluded.body_json, \
                 updated_at     = NOW()",
        )
        .bind(session_id.0.to_string())
        .bind(&principal_value)
        .bind(&body_value)
        .execute(pool)
        .await
        .map_err(sqlx_backend)?;
        Ok(())
    }

    async fn list(&self, principal: Principal) -> FlowResult<Vec<SessionId>> {
        let pool = self.pool.sqlx();
        // Principal identity is its `subject` (R3 auth);
        // `principal_json` is the persisted full envelope, so we
        // pull every row and filter by subject. Sessions are
        // small-cardinality per principal — a scan over a single
        // tenant's rows is fine. A future job can add a
        // `subject` column + index if a hot listing path
        // surfaces.
        let target = principal.subject.clone();
        let rows: Vec<(String, serde_json::Value)> =
            sqlx::query_as("SELECT session_id, principal_json FROM sessions ORDER BY created_at")
                .fetch_all(pool)
                .await
                .map_err(sqlx_backend)?;
        let mut out = Vec::new();
        for (sid, pj) in rows {
            let p: Principal = from_value("sessions.principal_json", pj)?;
            if p.subject == target {
                let uuid = uuid::Uuid::parse_str(&sid)
                    .map_err(|e| FlowError::Backend(format!("sessions.session_id: {e}")))?;
                out.push(SessionId(uuid));
            }
        }
        Ok(out)
    }
}

//! [`SqliteSessionStore`] — session records and principal-scoped
//! listing (D-F3.3).

use async_trait::async_trait;
use starter_flow_spi::flow::{FlowError, FlowResult, SessionId, SessionRecord, SessionStore};
use starter_flow_spi::Principal;

use super::schema::{from_json, sqlx_backend, to_json};
use crate::pool::Pool;

/// SQLite-backed [`SessionStore`].
#[derive(Clone)]
pub struct SqliteSessionStore {
    pool: Pool,
}

impl SqliteSessionStore {
    /// Construct a [`SqliteSessionStore`] over an existing
    /// [`Pool`]. Pair with [`super::FLOW_MIGRATION_SOURCE`].
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionStore for SqliteSessionStore {
    async fn get(&self, session_id: SessionId) -> FlowResult<Option<SessionRecord>> {
        let pool = self.pool.sqlx();
        let row: Option<(String, String)> =
            sqlx::query_as("SELECT principal_json, body_json FROM sessions WHERE session_id = ?1")
                .bind(session_id.0.to_string())
                .fetch_optional(pool)
                .await
                .map_err(sqlx_backend)?;
        let Some((principal_json, body_json)) = row else {
            return Ok(None);
        };
        let principal: Principal = from_json("sessions.principal_json", &principal_json)?;
        let body: serde_json::Value = from_json("sessions.body_json", &body_json)?;
        Ok(Some(SessionRecord::new(session_id, principal, body)))
    }

    async fn put(&self, session_id: SessionId, record: SessionRecord) -> FlowResult<()> {
        let pool = self.pool.sqlx();
        let principal_json = to_json(&record.principal)?;
        let body_json = to_json(&record.body)?;
        sqlx::query(
            "INSERT INTO sessions \
                 (session_id, principal_json, body_json, updated_at) \
                 VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP) \
             ON CONFLICT (session_id) DO UPDATE SET \
                 principal_json = excluded.principal_json, \
                 body_json      = excluded.body_json, \
                 updated_at     = CURRENT_TIMESTAMP",
        )
        .bind(session_id.0.to_string())
        .bind(&principal_json)
        .bind(&body_json)
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
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT session_id, principal_json FROM sessions ORDER BY created_at")
                .fetch_all(pool)
                .await
                .map_err(sqlx_backend)?;
        let mut out = Vec::new();
        for (sid, pj) in rows {
            let p: Principal = from_json("sessions.principal_json", &pj)?;
            if p.subject == target {
                let uuid = uuid::Uuid::parse_str(&sid)
                    .map_err(|e| FlowError::Backend(format!("sessions.session_id: {e}")))?;
                out.push(SessionId(uuid));
            }
        }
        Ok(out)
    }
}

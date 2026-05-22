//! [`PostgresAgentSessionStore`] — port of [`SqliteAgentSessionStore`]
//! to Postgres. Identical trait behaviour, identical caps, identical
//! M5 transactionality. Only the SQL dialect and column types differ.
//!
//! [`SqliteAgentSessionStore`]:
//!     starter_store_sqlite::flow::SqliteAgentSessionStore

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use starter_flow_spi::agent_session::{
    AgentSession, AgentSessionError, AgentSessionId, AgentSessionResult, AgentSessionStore,
    Artifact, ArtifactMeta, ArtifactWrite, PutArtifactError, RetentionPolicy, RetentionSweepReport,
    Turn, TurnInput, TurnReceipt, TurnRole, ARTIFACT_VALUE_CAP_BYTES, TURN_CONTENT_CAP_BYTES,
    TURN_SCHEMA_VERSION,
};

use crate::pool::Pool;

/// Postgres-backed [`AgentSessionStore`].
#[derive(Clone)]
pub struct PostgresAgentSessionStore {
    pool: Pool,
}

impl PostgresAgentSessionStore {
    /// Construct a [`PostgresAgentSessionStore`] over an existing
    /// [`Pool`]. Pair with [`super::AGENT_SESSION_MIGRATION_SOURCE`]
    /// on the migrate chain.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

fn backend(err: sqlx::Error) -> AgentSessionError {
    AgentSessionError::Backend(err.to_string())
}

fn put_backend(err: sqlx::Error) -> PutArtifactError {
    PutArtifactError::Backend(err.to_string())
}

fn role_to_str(role: TurnRole) -> &'static str {
    match role {
        TurnRole::User => "user",
        TurnRole::Assistant => "assistant",
        TurnRole::Tool => "tool",
        _ => "tool",
    }
}

fn role_from_str(s: &str) -> Result<TurnRole, AgentSessionError> {
    Ok(match s {
        "user" => TurnRole::User,
        "assistant" => TurnRole::Assistant,
        "tool" => TurnRole::Tool,
        other => {
            return Err(AgentSessionError::Backend(format!(
                "unknown turn role: {other}"
            )))
        }
    })
}

/// Pre-flight cap check that mirrors the SQLite impl. Returns the
/// serialised length so the caller can persist it in `content_bytes`
/// / `value_bytes` without re-serialising.
fn serialized_len(value: &serde_json::Value) -> Result<usize, AgentSessionError> {
    serde_json::to_vec(value)
        .map(|v| v.len())
        .map_err(|e| AgentSessionError::Backend(format!("serialize: {e}")))
}

#[async_trait]
impl AgentSessionStore for PostgresAgentSessionStore {
    async fn create(
        &self,
        id: AgentSessionId,
        kind: &str,
        owner: &str,
        metadata: serde_json::Value,
    ) -> AgentSessionResult<()> {
        let pool = self.pool.sqlx();
        sqlx::query(
            "INSERT INTO agent_sessions (id, kind, owner, metadata_json) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(id.0.to_string())
        .bind(kind)
        .bind(owner)
        .bind(metadata)
        .execute(pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn get(&self, id: AgentSessionId) -> AgentSessionResult<Option<AgentSession>> {
        let pool = self.pool.sqlx();
        let row = sqlx::query(
            "SELECT kind, owner, created_at, updated_at, metadata_json \
             FROM agent_sessions WHERE id = $1",
        )
        .bind(id.0.to_string())
        .fetch_optional(pool)
        .await
        .map_err(backend)?;
        let Some(row) = row else { return Ok(None) };
        Ok(Some(AgentSession::new(
            id,
            row.try_get::<String, _>("kind").map_err(backend)?,
            row.try_get::<String, _>("owner").map_err(backend)?,
            row.try_get::<DateTime<Utc>, _>("created_at")
                .map_err(backend)?,
            row.try_get::<DateTime<Utc>, _>("updated_at")
                .map_err(backend)?,
            row.try_get::<serde_json::Value, _>("metadata_json")
                .map_err(backend)?,
        )))
    }

    async fn delete(&self, id: AgentSessionId) -> AgentSessionResult<()> {
        let pool = self.pool.sqlx();
        sqlx::query("DELETE FROM agent_sessions WHERE id = $1")
            .bind(id.0.to_string())
            .execute(pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn append_turn_with_artifacts(
        &self,
        id: AgentSessionId,
        turn: TurnInput,
        artifacts: &[ArtifactWrite],
    ) -> AgentSessionResult<TurnReceipt> {
        // M12 / M8: pre-flight caps before opening a transaction.
        let content_bytes = serialized_len(&turn.content)?;
        if content_bytes > TURN_CONTENT_CAP_BYTES {
            return Err(AgentSessionError::TurnTooLarge {
                bytes: content_bytes,
                cap: TURN_CONTENT_CAP_BYTES,
            });
        }
        let mut artifact_bytes: Vec<usize> = Vec::with_capacity(artifacts.len());
        for a in artifacts {
            let bytes = serialized_len(&a.value)?;
            if bytes > ARTIFACT_VALUE_CAP_BYTES {
                return Err(AgentSessionError::ArtifactTooLarge {
                    key: a.key.clone(),
                    bytes,
                    cap: ARTIFACT_VALUE_CAP_BYTES,
                });
            }
            artifact_bytes.push(bytes);
        }

        let pool = self.pool.sqlx();
        let mut tx = pool.begin().await.map_err(backend)?;
        let session_id_s = id.0.to_string();

        // Lock the session row for the duration of the tx so two
        // concurrent appends serialise on the per-session `seq`
        // allocation (M5). `SELECT ... FOR UPDATE` is the Postgres
        // analogue of SQLite's `BEGIN IMMEDIATE`.
        let session_exists: Option<(String,)> =
            sqlx::query_as("SELECT id FROM agent_sessions WHERE id = $1 FOR UPDATE")
                .bind(&session_id_s)
                .fetch_optional(&mut *tx)
                .await
                .map_err(backend)?;
        if session_exists.is_none() {
            return Err(AgentSessionError::SessionNotFound(id));
        }

        let next_seq: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM agent_session_turns \
             WHERE session_id = $1",
        )
        .bind(&session_id_s)
        .fetch_one(&mut *tx)
        .await
        .map_err(backend)?;

        sqlx::query(
            "INSERT INTO agent_session_turns \
             (session_id, seq, role, content_json, schema_version, \
              content_bytes, tokens_in, tokens_out) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&session_id_s)
        .bind(next_seq)
        .bind(role_to_str(turn.role))
        .bind(&turn.content)
        .bind(TURN_SCHEMA_VERSION as i32)
        .bind(content_bytes as i32)
        .bind(turn.tokens_in.map(|t| t as i32))
        .bind(turn.tokens_out.map(|t| t as i32))
        .execute(&mut *tx)
        .await
        .map_err(backend)?;

        let mut artifact_versions: Vec<u32> = Vec::with_capacity(artifacts.len());
        for (a, value_bytes) in artifacts.iter().zip(artifact_bytes.iter()) {
            let next_version: i32 = sqlx::query_scalar(
                "SELECT COALESCE(MAX(version), 0) + 1 \
                 FROM agent_session_artifacts \
                 WHERE session_id = $1 AND key = $2",
            )
            .bind(&session_id_s)
            .bind(&a.key)
            .fetch_one(&mut *tx)
            .await
            .map_err(backend)?;

            sqlx::query(
                "INSERT INTO agent_session_artifacts \
                 (session_id, key, version, parent_version, \
                  value_json, value_bytes, produced_by_seq) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&session_id_s)
            .bind(&a.key)
            .bind(next_version)
            .bind(a.parent_version.map(|p| p as i32))
            .bind(&a.value)
            .bind(*value_bytes as i32)
            .bind(next_seq)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;

            artifact_versions.push(next_version as u32);
        }

        sqlx::query("UPDATE agent_sessions SET updated_at = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(&session_id_s)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;

        tx.commit().await.map_err(backend)?;
        Ok(TurnReceipt::new(next_seq as u32, artifact_versions))
    }

    async fn put_artifact_direct(
        &self,
        id: AgentSessionId,
        key: &str,
        value: serde_json::Value,
        expected_prev_version: Option<u32>,
    ) -> Result<u32, PutArtifactError> {
        let value_bytes = serde_json::to_vec(&value)
            .map(|v| v.len())
            .map_err(|e| PutArtifactError::Backend(e.to_string()))?;
        if value_bytes > ARTIFACT_VALUE_CAP_BYTES {
            return Err(PutArtifactError::TooLarge {
                bytes: value_bytes,
                cap: ARTIFACT_VALUE_CAP_BYTES,
            });
        }

        let pool = self.pool.sqlx();
        let mut tx = pool.begin().await.map_err(put_backend)?;
        let session_id_s = id.0.to_string();

        let session_exists: Option<(String,)> =
            sqlx::query_as("SELECT id FROM agent_sessions WHERE id = $1 FOR UPDATE")
                .bind(&session_id_s)
                .fetch_optional(&mut *tx)
                .await
                .map_err(put_backend)?;
        if session_exists.is_none() {
            return Err(PutArtifactError::SessionNotFound(id));
        }

        let current: Option<i32> = sqlx::query_scalar(
            "SELECT MAX(version) FROM agent_session_artifacts \
             WHERE session_id = $1 AND key = $2",
        )
        .bind(&session_id_s)
        .bind(key)
        .fetch_one(&mut *tx)
        .await
        .map_err(put_backend)?;
        let current_u32 = current.map(|v| v as u32).unwrap_or(0);

        if let Some(expected) = expected_prev_version {
            if expected != current_u32 {
                return Err(PutArtifactError::Conflict {
                    current: current_u32,
                });
            }
        }

        let next_version = current_u32.saturating_add(1);

        sqlx::query(
            "INSERT INTO agent_session_artifacts \
             (session_id, key, version, parent_version, \
              value_json, value_bytes, produced_by_seq) \
             VALUES ($1, $2, $3, $4, $5, $6, NULL)",
        )
        .bind(&session_id_s)
        .bind(key)
        .bind(next_version as i32)
        .bind(if current_u32 == 0 {
            None
        } else {
            Some(current_u32 as i32)
        })
        .bind(&value)
        .bind(value_bytes as i32)
        .execute(&mut *tx)
        .await
        .map_err(put_backend)?;

        sqlx::query("UPDATE agent_sessions SET updated_at = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(&session_id_s)
            .execute(&mut *tx)
            .await
            .map_err(put_backend)?;

        tx.commit().await.map_err(put_backend)?;
        Ok(next_version)
    }

    async fn list_turns(
        &self,
        id: AgentSessionId,
        since_seq: Option<u32>,
        limit: Option<u32>,
    ) -> AgentSessionResult<Vec<Turn>> {
        let pool = self.pool.sqlx();
        let session_id_s = id.0.to_string();
        let since = since_seq.unwrap_or(0) as i32;
        // Postgres `LIMIT` accepts NULL for "no cap" via `LIMIT ALL`;
        // we keep the bind set uniform by sending a huge sentinel.
        let cap = limit.map(|l| l as i64).unwrap_or(i64::MAX);

        let rows = sqlx::query(
            "SELECT seq, role, content_json, schema_version, \
                    content_bytes, tokens_in, tokens_out, created_at \
             FROM agent_session_turns \
             WHERE session_id = $1 AND seq > $2 \
             ORDER BY seq ASC \
             LIMIT $3",
        )
        .bind(&session_id_s)
        .bind(since)
        .bind(cap)
        .fetch_all(pool)
        .await
        .map_err(backend)?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let seq: i32 = row.try_get("seq").map_err(backend)?;
            let role_s: String = row.try_get("role").map_err(backend)?;
            let content_json: serde_json::Value = row.try_get("content_json").map_err(backend)?;
            let schema_version: i32 = row.try_get("schema_version").map_err(backend)?;
            let content_bytes: i32 = row.try_get("content_bytes").map_err(backend)?;
            let tokens_in: Option<i32> = row.try_get("tokens_in").map_err(backend)?;
            let tokens_out: Option<i32> = row.try_get("tokens_out").map_err(backend)?;
            let created_at: DateTime<Utc> = row.try_get("created_at").map_err(backend)?;

            out.push(Turn::new(
                id,
                seq as u32,
                role_from_str(&role_s)?,
                content_json,
                schema_version as u32,
                content_bytes as u32,
                tokens_in.map(|t| t as u32),
                tokens_out.map(|t| t as u32),
                created_at,
            ));
        }
        Ok(out)
    }

    async fn latest_artifact(
        &self,
        id: AgentSessionId,
        key: &str,
    ) -> AgentSessionResult<Option<Artifact>> {
        let pool = self.pool.sqlx();
        let row = sqlx::query(
            "SELECT version, parent_version, value_json, value_bytes, \
                    produced_by_seq, updated_at \
             FROM agent_session_artifacts \
             WHERE session_id = $1 AND key = $2 \
             ORDER BY version DESC \
             LIMIT 1",
        )
        .bind(id.0.to_string())
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(backend)?;
        let Some(row) = row else { return Ok(None) };
        row_to_artifact(&row, id, key.to_owned()).map(Some)
    }

    async fn artifact_at(
        &self,
        id: AgentSessionId,
        key: &str,
        version: u32,
    ) -> AgentSessionResult<Option<Artifact>> {
        let pool = self.pool.sqlx();
        let row = sqlx::query(
            "SELECT version, parent_version, value_json, value_bytes, \
                    produced_by_seq, updated_at \
             FROM agent_session_artifacts \
             WHERE session_id = $1 AND key = $2 AND version = $3",
        )
        .bind(id.0.to_string())
        .bind(key)
        .bind(version as i32)
        .fetch_optional(pool)
        .await
        .map_err(backend)?;
        let Some(row) = row else { return Ok(None) };
        row_to_artifact(&row, id, key.to_owned()).map(Some)
    }

    async fn list_artifact_versions(
        &self,
        id: AgentSessionId,
        key: &str,
    ) -> AgentSessionResult<Vec<ArtifactMeta>> {
        let pool = self.pool.sqlx();
        let rows = sqlx::query(
            "SELECT version, parent_version, value_bytes, produced_by_seq, updated_at \
             FROM agent_session_artifacts \
             WHERE session_id = $1 AND key = $2 \
             ORDER BY version DESC",
        )
        .bind(id.0.to_string())
        .bind(key)
        .fetch_all(pool)
        .await
        .map_err(backend)?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let version: i32 = row.try_get("version").map_err(backend)?;
            let parent_version: Option<i32> = row.try_get("parent_version").map_err(backend)?;
            let value_bytes: i32 = row.try_get("value_bytes").map_err(backend)?;
            let produced_by_seq: Option<i32> = row.try_get("produced_by_seq").map_err(backend)?;
            let updated_at: DateTime<Utc> = row.try_get("updated_at").map_err(backend)?;
            out.push(ArtifactMeta::new(
                id,
                key.to_owned(),
                version as u32,
                parent_version.map(|v| v as u32),
                value_bytes as u32,
                produced_by_seq.map(|s| s as u32),
                updated_at,
            ));
        }
        Ok(out)
    }

    async fn sweep_retention(
        &self,
        kind: &str,
        policy: &RetentionPolicy,
        now: DateTime<Utc>,
    ) -> AgentSessionResult<RetentionSweepReport> {
        match policy {
            RetentionPolicy::KeepForever => Ok(RetentionSweepReport::default()),
            RetentionPolicy::DeleteAfter { ttl } => {
                let cutoff = now - *ttl;
                let pool = self.pool.sqlx();
                let mut tx = pool.begin().await.map_err(backend)?;

                let session_ids: Vec<(String,)> = sqlx::query_as(
                    "SELECT id FROM agent_sessions \
                     WHERE kind = $1 AND updated_at < $2",
                )
                .bind(kind)
                .bind(cutoff)
                .fetch_all(&mut *tx)
                .await
                .map_err(backend)?;

                if session_ids.is_empty() {
                    return Ok(RetentionSweepReport::default());
                }

                // Count cascades up front. Using ANY($1::text[]) is
                // cleaner than building a placeholder list — same
                // semantics, one bind, plan-stable.
                let ids: Vec<String> = session_ids.iter().map(|(s,)| s.clone()).collect();
                let turns_deleted: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM agent_session_turns \
                     WHERE session_id = ANY($1)",
                )
                .bind(&ids)
                .fetch_one(&mut *tx)
                .await
                .map_err(backend)?;
                let artifacts_deleted: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM agent_session_artifacts \
                     WHERE session_id = ANY($1)",
                )
                .bind(&ids)
                .fetch_one(&mut *tx)
                .await
                .map_err(backend)?;

                let res = sqlx::query(
                    "DELETE FROM agent_sessions \
                     WHERE kind = $1 AND updated_at < $2",
                )
                .bind(kind)
                .bind(cutoff)
                .execute(&mut *tx)
                .await
                .map_err(backend)?;

                tx.commit().await.map_err(backend)?;

                Ok(RetentionSweepReport {
                    sessions_deleted: res.rows_affected(),
                    turns_deleted: turns_deleted as u64,
                    artifacts_deleted: artifacts_deleted as u64,
                })
            }
            RetentionPolicy::DeleteTurnsAfter {
                ttl,
                keep_latest_artifact,
            } => {
                let cutoff = now - *ttl;
                let pool = self.pool.sqlx();
                let mut tx = pool.begin().await.map_err(backend)?;

                // Pre-null the artifact back-pointer so the FK
                // `ON DELETE SET NULL` never tries to NULL a NOT
                // NULL `session_id` column (same mitigation as the
                // SQLite impl).
                sqlx::query(
                    "UPDATE agent_session_artifacts a \
                     SET produced_by_seq = NULL \
                     WHERE a.produced_by_seq IS NOT NULL \
                       AND a.session_id IN (SELECT id FROM agent_sessions WHERE kind = $1) \
                       AND EXISTS ( \
                            SELECT 1 FROM agent_session_turns t \
                            WHERE t.session_id = a.session_id \
                              AND t.seq = a.produced_by_seq \
                              AND t.created_at < $2 \
                       )",
                )
                .bind(kind)
                .bind(cutoff)
                .execute(&mut *tx)
                .await
                .map_err(backend)?;

                let turns_res = sqlx::query(
                    "DELETE FROM agent_session_turns \
                     WHERE session_id IN (SELECT id FROM agent_sessions WHERE kind = $1) \
                       AND created_at < $2",
                )
                .bind(kind)
                .bind(cutoff)
                .execute(&mut *tx)
                .await
                .map_err(backend)?;

                let mut artifacts_deleted: u64 = 0;
                if *keep_latest_artifact {
                    let res = sqlx::query(
                        "DELETE FROM agent_session_artifacts a \
                         WHERE a.session_id IN (SELECT id FROM agent_sessions WHERE kind = $1) \
                           AND a.updated_at < $2 \
                           AND a.version < ( \
                                SELECT MAX(b.version) \
                                FROM agent_session_artifacts b \
                                WHERE b.session_id = a.session_id AND b.key = a.key \
                           )",
                    )
                    .bind(kind)
                    .bind(cutoff)
                    .execute(&mut *tx)
                    .await
                    .map_err(backend)?;
                    artifacts_deleted = res.rows_affected();
                }

                tx.commit().await.map_err(backend)?;

                Ok(RetentionSweepReport {
                    sessions_deleted: 0,
                    turns_deleted: turns_res.rows_affected(),
                    artifacts_deleted,
                })
            }
        }
    }
}

fn row_to_artifact(
    row: &sqlx::postgres::PgRow,
    session_id: AgentSessionId,
    key: String,
) -> AgentSessionResult<Artifact> {
    let version: i32 = row.try_get("version").map_err(backend)?;
    let parent_version: Option<i32> = row.try_get("parent_version").map_err(backend)?;
    let value: serde_json::Value = row.try_get("value_json").map_err(backend)?;
    let value_bytes: i32 = row.try_get("value_bytes").map_err(backend)?;
    let produced_by_seq: Option<i32> = row.try_get("produced_by_seq").map_err(backend)?;
    let updated_at: DateTime<Utc> = row.try_get("updated_at").map_err(backend)?;
    Ok(Artifact::new(
        session_id,
        key,
        version as u32,
        parent_version.map(|v| v as u32),
        value,
        value_bytes as u32,
        produced_by_seq.map(|s| s as u32),
        updated_at,
    ))
}

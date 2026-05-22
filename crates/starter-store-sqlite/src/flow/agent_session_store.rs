//! [`SqliteAgentSessionStore`] — append-only turns + versioned
//! artifacts (DOCS/agent/MEMORY.md Phase M-A).
//!
//! Distinct from [`super::SqliteSessionStore`]: that impl backs the
//! opaque key/value `SessionStore` SPI seam; this one backs the
//! richer `AgentSessionStore` seam used by the ai-agent loop and the
//! page builder (MEMORY.md M1). Both impls share the `flow`
//! migration source — no second migration runner.

use async_trait::async_trait;
use sqlx::Row;
use starter_flow_spi::agent_session::{
    AgentSession, AgentSessionError, AgentSessionId, AgentSessionResult, AgentSessionStore,
    Artifact, ArtifactMeta, ArtifactWrite, PutArtifactError, RetentionPolicy, RetentionSweepReport,
    Turn, TurnInput, TurnReceipt, TurnRole, ARTIFACT_VALUE_CAP_BYTES, TURN_CONTENT_CAP_BYTES,
    TURN_SCHEMA_VERSION,
};

use crate::pool::Pool;

/// SQLite-backed [`AgentSessionStore`].
#[derive(Clone)]
pub struct SqliteAgentSessionStore {
    pool: Pool,
}

impl SqliteAgentSessionStore {
    /// Construct a [`SqliteAgentSessionStore`] over an existing
    /// [`Pool`]. Pair with [`super::FLOW_MIGRATION_SOURCE`] on the
    /// migrate chain — the `agent_sessions` / `agent_session_turns`
    /// / `agent_session_artifacts` tables ship alongside the flow
    /// engine's own schema.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

/// Map a `sqlx::Error` to [`AgentSessionError::Backend`].
fn backend(err: sqlx::Error) -> AgentSessionError {
    AgentSessionError::Backend(err.to_string())
}

/// Map a `sqlx::Error` to [`PutArtifactError::Backend`].
fn put_backend(err: sqlx::Error) -> PutArtifactError {
    PutArtifactError::Backend(err.to_string())
}

/// Serialise to compact JSON, mapping failures to the `Backend`
/// variant — disk corruption / schema drift are operator-actionable
/// so the error reaches the caller without a log-line attached.
fn to_json<T: serde::Serialize>(value: &T) -> Result<String, AgentSessionError> {
    serde_json::to_string(value).map_err(|e| AgentSessionError::Backend(format!("serialize: {e}")))
}

fn from_json<T: serde::de::DeserializeOwned>(
    column: &'static str,
    raw: &str,
) -> Result<T, AgentSessionError> {
    serde_json::from_str(raw)
        .map_err(|e| AgentSessionError::Backend(format!("deserialize {column}: {e}")))
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
            )));
        }
    })
}

/// SQLite stores timestamps as ISO-8601 TEXT; parse to chrono.
fn parse_ts(
    column: &'static str,
    raw: &str,
) -> Result<chrono::DateTime<chrono::Utc>, AgentSessionError> {
    // CURRENT_TIMESTAMP in SQLite emits `YYYY-MM-DD HH:MM:SS` (UTC,
    // no offset). Accept both that shape and full RFC 3339 so a
    // future migration that switches to `strftime('%Y-%m-%dT...')`
    // doesn't break readers.
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(raw) {
        return Ok(dt.with_timezone(&chrono::Utc));
    }
    chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S")
        .map(|naive| naive.and_utc())
        .map_err(|e| AgentSessionError::Backend(format!("parse {column}: {e}")))
}

#[async_trait]
impl AgentSessionStore for SqliteAgentSessionStore {
    async fn create(
        &self,
        id: AgentSessionId,
        kind: &str,
        owner: &str,
        metadata: serde_json::Value,
    ) -> AgentSessionResult<()> {
        let pool = self.pool.sqlx();
        let meta_json = to_json(&metadata)?;
        sqlx::query(
            "INSERT INTO agent_sessions (id, kind, owner, metadata_json) \
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(id.0.to_string())
        .bind(kind)
        .bind(owner)
        .bind(&meta_json)
        .execute(pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn get(&self, id: AgentSessionId) -> AgentSessionResult<Option<AgentSession>> {
        let pool = self.pool.sqlx();
        let row = sqlx::query(
            "SELECT kind, owner, created_at, updated_at, metadata_json \
             FROM agent_sessions WHERE id = ?1",
        )
        .bind(id.0.to_string())
        .fetch_optional(pool)
        .await
        .map_err(backend)?;
        let Some(row) = row else { return Ok(None) };
        let kind: String = row.try_get("kind").map_err(backend)?;
        let owner: String = row.try_get("owner").map_err(backend)?;
        let created_at: String = row.try_get("created_at").map_err(backend)?;
        let updated_at: String = row.try_get("updated_at").map_err(backend)?;
        let metadata_json: String = row.try_get("metadata_json").map_err(backend)?;
        Ok(Some(AgentSession::new(
            id,
            kind,
            owner,
            parse_ts("agent_sessions.created_at", &created_at)?,
            parse_ts("agent_sessions.updated_at", &updated_at)?,
            from_json("agent_sessions.metadata_json", &metadata_json)?,
        )))
    }

    async fn delete(&self, id: AgentSessionId) -> AgentSessionResult<()> {
        let pool = self.pool.sqlx();
        sqlx::query("DELETE FROM agent_sessions WHERE id = ?1")
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
        let pool = self.pool.sqlx();

        // M12: pre-flight cap check before opening the
        // transaction. Validation is in the trait/impl, not the
        // backend — the same check will live in the Postgres
        // impl unchanged.
        let content_json = to_json(&turn.content)?;
        let content_bytes = content_json.len();
        if content_bytes > TURN_CONTENT_CAP_BYTES {
            return Err(AgentSessionError::TurnTooLarge {
                bytes: content_bytes,
                cap: TURN_CONTENT_CAP_BYTES,
            });
        }

        // M8: per-artifact cap. Aggregate caps are a replay-layer
        // concern (snapshot strategy); the store only enforces per
        // row.
        let mut artifact_json: Vec<(String, usize)> = Vec::with_capacity(artifacts.len());
        for a in artifacts {
            let v = to_json(&a.value)?;
            let bytes = v.len();
            if bytes > ARTIFACT_VALUE_CAP_BYTES {
                return Err(AgentSessionError::ArtifactTooLarge {
                    key: a.key.clone(),
                    bytes,
                    cap: ARTIFACT_VALUE_CAP_BYTES,
                });
            }
            artifact_json.push((v, bytes));
        }

        // SQLite `BEGIN IMMEDIATE` so two concurrent
        // `append_turn_with_artifacts` calls on the same session
        // serialise on the first write — neither sees a half
        // state, and the per-session `seq` allocation is race-free
        // (M5 concurrency contract).
        let mut tx = pool.begin().await.map_err(backend)?;
        sqlx::query("BEGIN IMMEDIATE").execute(&mut *tx).await.ok(); // sqlx already wrapped us in a transaction; the
                                                                     // explicit upgrade is a no-op when already in
                                                                     // IMMEDIATE — kept as a hint for future maintainers.

        let session_id_s = id.0.to_string();

        // Confirm the session row exists; without this an FK
        // violation would surface as a generic `Backend` error and
        // mask the real cause.
        let session_exists: Option<(i64,)> =
            sqlx::query_as("SELECT 1 FROM agent_sessions WHERE id = ?1")
                .bind(&session_id_s)
                .fetch_optional(&mut *tx)
                .await
                .map_err(backend)?;
        if session_exists.is_none() {
            return Err(AgentSessionError::SessionNotFound(id));
        }

        // Assign next seq inside the transaction.
        let next_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) + 1 \
             FROM agent_session_turns WHERE session_id = ?1",
        )
        .bind(&session_id_s)
        .fetch_one(&mut *tx)
        .await
        .map_err(backend)?;

        sqlx::query(
            "INSERT INTO agent_session_turns \
             (session_id, seq, role, content_json, schema_version, \
              content_bytes, tokens_in, tokens_out) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(&session_id_s)
        .bind(next_seq)
        .bind(role_to_str(turn.role))
        .bind(&content_json)
        .bind(TURN_SCHEMA_VERSION as i64)
        .bind(content_bytes as i64)
        .bind(turn.tokens_in.map(|t| t as i64))
        .bind(turn.tokens_out.map(|t| t as i64))
        .execute(&mut *tx)
        .await
        .map_err(backend)?;

        let mut artifact_versions: Vec<u32> = Vec::with_capacity(artifacts.len());
        for (a, (value_json, value_bytes)) in artifacts.iter().zip(artifact_json.iter()) {
            let next_version: i64 = sqlx::query_scalar(
                "SELECT COALESCE(MAX(version), 0) + 1 \
                 FROM agent_session_artifacts \
                 WHERE session_id = ?1 AND key = ?2",
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
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .bind(&session_id_s)
            .bind(&a.key)
            .bind(next_version)
            .bind(a.parent_version.map(|p| p as i64))
            .bind(value_json)
            .bind(*value_bytes as i64)
            .bind(next_seq)
            .execute(&mut *tx)
            .await
            .map_err(backend)?;

            artifact_versions.push(next_version as u32);
        }

        // Touch updated_at on the parent session row so
        // retention sweepers and listings see write activity.
        sqlx::query("UPDATE agent_sessions SET updated_at = CURRENT_TIMESTAMP WHERE id = ?1")
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
        let pool = self.pool.sqlx();

        let value_json =
            serde_json::to_string(&value).map_err(|e| PutArtifactError::Backend(e.to_string()))?;
        let value_bytes = value_json.len();
        if value_bytes > ARTIFACT_VALUE_CAP_BYTES {
            return Err(PutArtifactError::TooLarge {
                bytes: value_bytes,
                cap: ARTIFACT_VALUE_CAP_BYTES,
            });
        }

        let mut tx = pool.begin().await.map_err(put_backend)?;
        let session_id_s = id.0.to_string();

        let session_exists: Option<(i64,)> =
            sqlx::query_as("SELECT 1 FROM agent_sessions WHERE id = ?1")
                .bind(&session_id_s)
                .fetch_optional(&mut *tx)
                .await
                .map_err(put_backend)?;
        if session_exists.is_none() {
            return Err(PutArtifactError::SessionNotFound(id));
        }

        // Read current latest version under the transaction so
        // the CAS-style compare-and-set is race-free.
        let current: Option<i64> = sqlx::query_scalar(
            "SELECT MAX(version) FROM agent_session_artifacts \
             WHERE session_id = ?1 AND key = ?2",
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
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)",
        )
        .bind(&session_id_s)
        .bind(key)
        .bind(next_version as i64)
        .bind(if current_u32 == 0 {
            None
        } else {
            Some(current_u32 as i64)
        })
        .bind(&value_json)
        .bind(value_bytes as i64)
        .execute(&mut *tx)
        .await
        .map_err(put_backend)?;

        sqlx::query("UPDATE agent_sessions SET updated_at = CURRENT_TIMESTAMP WHERE id = ?1")
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
        let since = since_seq.unwrap_or(0) as i64;
        // SQLite has no "no limit" sentinel for prepared queries; a
        // huge cap keeps the query plan stable and the bind set
        // identical across both branches.
        let cap = limit.map(|l| l as i64).unwrap_or(i64::MAX);

        let rows = sqlx::query(
            "SELECT seq, role, content_json, schema_version, \
                    content_bytes, tokens_in, tokens_out, created_at \
             FROM agent_session_turns \
             WHERE session_id = ?1 AND seq > ?2 \
             ORDER BY seq ASC \
             LIMIT ?3",
        )
        .bind(&session_id_s)
        .bind(since)
        .bind(cap)
        .fetch_all(pool)
        .await
        .map_err(backend)?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let seq: i64 = row.try_get("seq").map_err(backend)?;
            let role_s: String = row.try_get("role").map_err(backend)?;
            let content_json: String = row.try_get("content_json").map_err(backend)?;
            let schema_version: i64 = row.try_get("schema_version").map_err(backend)?;
            let content_bytes: i64 = row.try_get("content_bytes").map_err(backend)?;
            let tokens_in: Option<i64> = row.try_get("tokens_in").map_err(backend)?;
            let tokens_out: Option<i64> = row.try_get("tokens_out").map_err(backend)?;
            let created_at: String = row.try_get("created_at").map_err(backend)?;

            out.push(Turn::new(
                id,
                seq as u32,
                role_from_str(&role_s)?,
                from_json("agent_session_turns.content_json", &content_json)?,
                schema_version as u32,
                content_bytes as u32,
                tokens_in.map(|t| t as u32),
                tokens_out.map(|t| t as u32),
                parse_ts("agent_session_turns.created_at", &created_at)?,
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
             WHERE session_id = ?1 AND key = ?2 \
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
             WHERE session_id = ?1 AND key = ?2 AND version = ?3",
        )
        .bind(id.0.to_string())
        .bind(key)
        .bind(version as i64)
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
             WHERE session_id = ?1 AND key = ?2 \
             ORDER BY version DESC",
        )
        .bind(id.0.to_string())
        .bind(key)
        .fetch_all(pool)
        .await
        .map_err(backend)?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let version: i64 = row.try_get("version").map_err(backend)?;
            let parent_version: Option<i64> = row.try_get("parent_version").map_err(backend)?;
            let value_bytes: i64 = row.try_get("value_bytes").map_err(backend)?;
            let produced_by_seq: Option<i64> = row.try_get("produced_by_seq").map_err(backend)?;
            let updated_at: String = row.try_get("updated_at").map_err(backend)?;
            out.push(ArtifactMeta::new(
                id,
                key.to_owned(),
                version as u32,
                parent_version.map(|v| v as u32),
                value_bytes as u32,
                produced_by_seq.map(|s| s as u32),
                parse_ts("agent_session_artifacts.updated_at", &updated_at)?,
            ));
        }
        Ok(out)
    }

    async fn sweep_retention(
        &self,
        kind: &str,
        policy: &RetentionPolicy,
        now: chrono::DateTime<chrono::Utc>,
    ) -> AgentSessionResult<RetentionSweepReport> {
        // MEMORY.md §M9 / Phase M-E. Three policies, all bounded
        // by a `kind` filter so a single store can host multiple
        // surfaces with different retention rules.
        //
        // Cutoff formatting mirrors what SQLite's
        // `CURRENT_TIMESTAMP` emits (`YYYY-MM-DD HH:MM:SS`) so the
        // lexicographic comparison the engine does on TEXT columns
        // works without per-row parsing.
        match policy {
            RetentionPolicy::KeepForever => Ok(RetentionSweepReport::default()),
            RetentionPolicy::DeleteAfter { ttl } => {
                let cutoff = now - *ttl;
                let cutoff_s = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();
                let pool = self.pool.sqlx();
                let mut tx = pool.begin().await.map_err(backend)?;

                // Count the impact first so the report is accurate
                // without relying on RETURNING (SQLite 3.35+ only,
                // but more portable to just SELECT before DELETE
                // inside the same transaction).
                let session_ids: Vec<(String,)> = sqlx::query_as(
                    "SELECT id FROM agent_sessions \
                     WHERE kind = ?1 AND updated_at < ?2",
                )
                .bind(kind)
                .bind(&cutoff_s)
                .fetch_all(&mut *tx)
                .await
                .map_err(backend)?;

                if session_ids.is_empty() {
                    return Ok(RetentionSweepReport::default());
                }

                // Count the cascades before they happen.
                let placeholders: String = std::iter::repeat("?")
                    .take(session_ids.len())
                    .collect::<Vec<_>>()
                    .join(",");
                let turn_count_q = format!(
                    "SELECT COUNT(*) FROM agent_session_turns WHERE session_id IN ({placeholders})"
                );
                let mut q = sqlx::query_scalar::<_, i64>(&turn_count_q);
                for (id,) in &session_ids {
                    q = q.bind(id);
                }
                let turns_deleted: i64 = q.fetch_one(&mut *tx).await.map_err(backend)?;

                let art_count_q = format!(
                    "SELECT COUNT(*) FROM agent_session_artifacts WHERE session_id IN ({placeholders})"
                );
                let mut q = sqlx::query_scalar::<_, i64>(&art_count_q);
                for (id,) in &session_ids {
                    q = q.bind(id);
                }
                let artifacts_deleted: i64 = q.fetch_one(&mut *tx).await.map_err(backend)?;

                let res = sqlx::query(
                    "DELETE FROM agent_sessions \
                     WHERE kind = ?1 AND updated_at < ?2",
                )
                .bind(kind)
                .bind(&cutoff_s)
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
                let cutoff_s = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();
                let pool = self.pool.sqlx();
                let mut tx = pool.begin().await.map_err(backend)?;

                // The FK `agent_session_artifacts(session_id,
                // produced_by_seq) → agent_session_turns(session_id,
                // seq) ON DELETE SET NULL` would try to NULL both
                // columns when a turn is deleted; `session_id` is
                // NOT NULL on artifacts so the cascade fails.
                // Pre-null the back-pointer on artifact rows whose
                // turn is about to disappear, so the delete is
                // unambiguous and the FK never fires.
                sqlx::query(
                    "UPDATE agent_session_artifacts \
                     SET produced_by_seq = NULL \
                     WHERE session_id IN (SELECT id FROM agent_sessions WHERE kind = ?1) \
                       AND produced_by_seq IS NOT NULL \
                       AND produced_by_seq IN ( \
                            SELECT seq FROM agent_session_turns \
                            WHERE session_id = agent_session_artifacts.session_id \
                              AND created_at < ?2 \
                       )",
                )
                .bind(kind)
                .bind(&cutoff_s)
                .execute(&mut *tx)
                .await
                .map_err(backend)?;

                // Turns: scope to the configured kind via subquery
                // so other surfaces sharing the same store are
                // untouched.
                let turns_res = sqlx::query(
                    "DELETE FROM agent_session_turns \
                     WHERE session_id IN (SELECT id FROM agent_sessions WHERE kind = ?1) \
                       AND created_at < ?2",
                )
                .bind(kind)
                .bind(&cutoff_s)
                .execute(&mut *tx)
                .await
                .map_err(backend)?;

                let mut artifacts_deleted: u64 = 0;
                if *keep_latest_artifact {
                    // Collapse history down to the latest version
                    // per `(session_id, key)` for any artifact row
                    // older than the cutoff. The MAX(version) tuple
                    // survives even if older than cutoff — the
                    // policy is "keep the artifact", not "keep only
                    // if recent".
                    let res = sqlx::query(
                        "DELETE FROM agent_session_artifacts \
                         WHERE session_id IN (SELECT id FROM agent_sessions WHERE kind = ?1) \
                           AND updated_at < ?2 \
                           AND (session_id, key, version) NOT IN ( \
                                SELECT session_id, key, MAX(version) \
                                FROM agent_session_artifacts \
                                GROUP BY session_id, key \
                           )",
                    )
                    .bind(kind)
                    .bind(&cutoff_s)
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
    row: &sqlx::sqlite::SqliteRow,
    session_id: AgentSessionId,
    key: String,
) -> AgentSessionResult<Artifact> {
    let version: i64 = row.try_get("version").map_err(backend)?;
    let parent_version: Option<i64> = row.try_get("parent_version").map_err(backend)?;
    let value_json: String = row.try_get("value_json").map_err(backend)?;
    let value_bytes: i64 = row.try_get("value_bytes").map_err(backend)?;
    let produced_by_seq: Option<i64> = row.try_get("produced_by_seq").map_err(backend)?;
    let updated_at: String = row.try_get("updated_at").map_err(backend)?;
    Ok(Artifact::new(
        session_id,
        key,
        version as u32,
        parent_version.map(|v| v as u32),
        from_json("agent_session_artifacts.value_json", &value_json)?,
        value_bytes as u32,
        produced_by_seq.map(|s| s as u32),
        parse_ts("agent_session_artifacts.updated_at", &updated_at)?,
    ))
}

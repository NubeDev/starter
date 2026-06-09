//! Update agents and advance session state.

use sqlx::PgPool;
use starter_spi::Error;
use uuid::Uuid;

use super::record::AgentPatch;
use crate::tenant_tx;

/// Apply a partial update to an agent. `None` fields are left unchanged;
/// `system_prompt: Some(None)` clears the prompt. Returns whether a row matched.
pub async fn update(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    patch: &AgentPatch,
) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    // COALESCE keeps the existing value when the bound parameter is NULL, except
    // system_prompt which is set directly so it can be cleared to NULL.
    let result = sqlx::query(
        "UPDATE nexus_agents SET \
            name          = COALESCE($2, name), \
            backend       = COALESCE($3, backend), \
            model         = COALESCE($4, model), \
            system_prompt = CASE WHEN $5 THEN $6 ELSE system_prompt END, \
            config        = COALESCE($7, config) \
         WHERE id = $1",
    )
    .bind(id)
    .bind(patch.name.as_ref())
    .bind(patch.backend.as_ref())
    .bind(patch.model.as_ref())
    .bind(patch.system_prompt.is_some())
    .bind(patch.system_prompt.as_ref().and_then(|o| o.as_ref()))
    .bind(patch.config.as_ref())
    .execute(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(result.rows_affected() > 0)
}

/// Advance a session's lifecycle state and bump `updated_at`.
pub async fn set_session_status(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    status: &str,
) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let result = sqlx::query(
        "UPDATE nexus_agent_sessions SET status = $2, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(result.rows_affected() > 0)
}

/// Replace a session's transcript with the accumulated messages and bump
/// `updated_at`. Called as a run progresses and on completion.
pub async fn set_session_transcript(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    transcript: &serde_json::Value,
) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let result = sqlx::query(
        "UPDATE nexus_agent_sessions SET transcript = $2, updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(transcript)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(result.rows_affected() > 0)
}

fn conflict_or_internal(e: sqlx::Error) -> Error {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            return Error::Conflict {
                message: "an agent with that name already exists".into(),
            };
        }
    }
    internal(e)
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}

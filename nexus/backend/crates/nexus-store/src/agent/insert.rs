//! Insert agents and sessions owned by a tenant.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{AgentRecord, NewAgent, NewSession, SessionRecord};
use crate::tenant_tx;

/// Insert a new agent. A name already used in the tenant is a `Conflict`, mirror
/// of the flow-name rule.
pub async fn insert(pool: &PgPool, tenant_id: &str, new: &NewAgent) -> Result<AgentRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_agents (tenant_id, name, backend, model, system_prompt, config) \
         VALUES ($1,$2,$3,$4,$5,$6) RETURNING id",
    )
    .bind(tenant_id)
    .bind(&new.name)
    .bind(&new.backend)
    .bind(&new.model)
    .bind(&new.system_prompt)
    .bind(&new.config)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(AgentRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: tenant_id.to_string(),
        name: new.name.clone(),
        backend: new.backend.clone(),
        model: new.model.clone(),
        system_prompt: new.system_prompt.clone(),
        config: new.config.clone(),
    })
}

/// Open a new session against an agent. The agent must exist in the tenant; the
/// FK plus RLS enforce that, so an unknown agent id is a `NotFound`.
pub async fn insert_session(
    pool: &PgPool,
    tenant_id: &str,
    new: &NewSession,
) -> Result<SessionRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_agent_sessions (tenant_id, agent_id, transcript) \
         VALUES ($1,$2,$3) RETURNING id, status",
    )
    .bind(tenant_id)
    .bind(new.agent_id)
    .bind(&new.transcript)
    .fetch_one(&mut *tx)
    .await
    .map_err(fk_or_internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(SessionRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: tenant_id.to_string(),
        agent_id: new.agent_id,
        status: row.get::<String, _>("status"),
        transcript: new.transcript.clone(),
    })
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

fn fk_or_internal(e: sqlx::Error) -> Error {
    if let sqlx::Error::Database(db) = &e {
        if db.is_foreign_key_violation() {
            return Error::NotFound {
                what: "agent".into(),
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

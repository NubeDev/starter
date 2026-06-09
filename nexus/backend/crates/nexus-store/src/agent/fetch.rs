//! List and get agents and sessions within a tenant.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{AgentRecord, SessionRecord};
use crate::tenant_tx;

/// List the tenant's agents, newest first.
pub async fn list(pool: &PgPool, tenant_id: &str) -> Result<Vec<AgentRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT id, tenant_id, name, backend, model, system_prompt, config \
         FROM nexus_agents ORDER BY created_at DESC",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_agent).collect())
}

/// Fetch one agent by id within the tenant. `Ok(None)` covers both absent and
/// another tenant's, so existence is not leaked.
pub async fn get(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<Option<AgentRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "SELECT id, tenant_id, name, backend, model, system_prompt, config \
         FROM nexus_agents WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.as_ref().map(row_to_agent))
}

/// List the sessions for one agent within the tenant, newest first.
pub async fn list_sessions(
    pool: &PgPool,
    tenant_id: &str,
    agent_id: Uuid,
) -> Result<Vec<SessionRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT id, tenant_id, agent_id, status, transcript \
         FROM nexus_agent_sessions WHERE agent_id = $1 ORDER BY created_at DESC",
    )
    .bind(agent_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_session).collect())
}

/// Fetch one session by id within the tenant.
pub async fn get_session(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
) -> Result<Option<SessionRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "SELECT id, tenant_id, agent_id, status, transcript \
         FROM nexus_agent_sessions WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.as_ref().map(row_to_session))
}

fn row_to_agent(row: &sqlx::postgres::PgRow) -> AgentRecord {
    AgentRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: row.get::<String, _>("tenant_id"),
        name: row.get::<String, _>("name"),
        backend: row.get::<String, _>("backend"),
        model: row.get::<String, _>("model"),
        system_prompt: row.get::<Option<String>, _>("system_prompt"),
        config: row.get::<serde_json::Value, _>("config"),
    }
}

fn row_to_session(row: &sqlx::postgres::PgRow) -> SessionRecord {
    SessionRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: row.get::<String, _>("tenant_id"),
        agent_id: row.get::<Uuid, _>("agent_id"),
        status: row.get::<String, _>("status"),
        transcript: row.get::<serde_json::Value, _>("transcript"),
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}

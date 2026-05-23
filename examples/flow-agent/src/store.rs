//! Postgres-backed repositories. Thin sqlx, no business logic.
//!
//! Tables live in `_sqlx_migrations_flow_agent`-managed migrations
//! (see [`crate::migrations`]). Optimistic locking on flows uses the
//! `version` column — every successful PUT bumps it and the next PUT
//! must present the previous value.

// The sqlx tuple shapes are deliberately literal — extracting a `type
// alias` per query would add a layer of indirection between schema
// and call site for no gain. Allowed at the file level.
#![allow(clippy::type_complexity)]

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::{
    Agent, AgentSummary, CreateAgent, CreateFlow, DomainError, Flow, FlowSummary, Run, UpdateAgent,
    UpdateFlow,
};

/// `(id, name, description, graph_json, version, created_at, updated_at)`
type FlowRow = (
    String,
    String,
    Option<String>,
    serde_json::Value,
    i64,
    DateTime<Utc>,
    DateTime<Utc>,
);

/// `(id, name, provider, model, system_prompt, tools_json, created_at, updated_at)`
type AgentRow = (
    String,
    String,
    String,
    String,
    Option<String>,
    serde_json::Value,
    DateTime<Utc>,
    DateTime<Utc>,
);

/// `(id, flow_id, status, started_at, finished_at, trace_json)`
type RunRow = (
    String,
    String,
    String,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
    Option<serde_json::Value>,
);

#[derive(Clone)]
pub struct FlowStore {
    pool: PgPool,
}

impl FlowStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<FlowSummary>, DomainError> {
        let rows: Vec<(String, String, Option<String>, i64, DateTime<Utc>)> = sqlx::query_as(
            "SELECT id, name, description, version, updated_at \
             FROM flows ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, name, description, version, updated_at)| FlowSummary {
                id,
                name,
                description,
                version,
                updated_at,
            })
            .collect())
    }

    pub async fn get(&self, id: &str) -> Result<Flow, DomainError> {
        let row: Option<FlowRow> = sqlx::query_as(
            "SELECT id, name, description, graph_json, version, created_at, updated_at \
             FROM flows WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some((id, name, description, graph_json, version, created_at, updated_at)) => {
                Ok(Flow {
                    id,
                    name,
                    description,
                    graph: serde_json::from_value(graph_json)?,
                    version,
                    created_at,
                    updated_at,
                })
            }
            None => Err(DomainError::NotFound(id.to_owned())),
        }
    }

    pub async fn create(&self, body: CreateFlow) -> Result<Flow, DomainError> {
        let id = crate::domain::new_id("flow");
        let now = Utc::now();
        let graph = body.graph.unwrap_or_else(empty_graph);
        sqlx::query(
            "INSERT INTO flows (id, name, description, graph_json, version, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, 1, $5, $5)",
        )
        .bind(&id)
        .bind(&body.name)
        .bind(&body.description)
        .bind(&graph)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(Flow {
            id,
            name: body.name,
            description: body.description,
            graph,
            version: 1,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn update(&self, id: &str, body: UpdateFlow) -> Result<Flow, DomainError> {
        let now = Utc::now();
        let res = sqlx::query(
            "UPDATE flows SET name = $1, description = $2, graph_json = $3, \
                              version = version + 1, updated_at = $4 \
             WHERE id = $5 AND version = $6",
        )
        .bind(&body.name)
        .bind(&body.description)
        .bind(&body.graph)
        .bind(now)
        .bind(id)
        .bind(body.version)
        .execute(&self.pool)
        .await?;
        if res.rows_affected() == 0 {
            // Distinguish not-found from version conflict.
            let exists: Option<(i64,)> =
                sqlx::query_as("SELECT version FROM flows WHERE id = $1")
                    .bind(id)
                    .fetch_optional(&self.pool)
                    .await?;
            return Err(if exists.is_some() {
                DomainError::VersionConflict
            } else {
                DomainError::NotFound(id.to_owned())
            });
        }
        self.get(id).await
    }

    pub async fn delete(&self, id: &str) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM flows WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(DomainError::NotFound(id.to_owned()));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct AgentStore {
    pool: PgPool,
}

impl AgentStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<AgentSummary>, DomainError> {
        let rows: Vec<(String, String, String, String, DateTime<Utc>)> = sqlx::query_as(
            "SELECT id, name, provider, model, updated_at \
             FROM agents ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, name, provider, model, updated_at)| AgentSummary {
                id,
                name,
                provider,
                model,
                updated_at,
            })
            .collect())
    }

    pub async fn get(&self, id: &str) -> Result<Agent, DomainError> {
        let row: Option<AgentRow> = sqlx::query_as(
            "SELECT id, name, provider, model, system_prompt, tools_json, created_at, updated_at \
             FROM agents WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some((
                id,
                name,
                provider,
                model,
                system_prompt,
                tools_json,
                created_at,
                updated_at,
            )) => Ok(Agent {
                id,
                name,
                provider,
                model,
                system_prompt,
                tools: serde_json::from_value(tools_json)?,
                created_at,
                updated_at,
            }),
            None => Err(DomainError::NotFound(id.to_owned())),
        }
    }

    pub async fn create(&self, body: CreateAgent) -> Result<Agent, DomainError> {
        let id = crate::domain::new_id("agent");
        let now = Utc::now();
        let tools_json = serde_json::to_value(&body.tools)?;
        sqlx::query(
            "INSERT INTO agents (id, name, provider, model, system_prompt, tools_json, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $7)",
        )
        .bind(&id)
        .bind(&body.name)
        .bind(&body.provider)
        .bind(&body.model)
        .bind(&body.system_prompt)
        .bind(&tools_json)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(Agent {
            id,
            name: body.name,
            provider: body.provider,
            model: body.model,
            system_prompt: body.system_prompt,
            tools: body.tools,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn update(&self, id: &str, body: UpdateAgent) -> Result<Agent, DomainError> {
        let now = Utc::now();
        let tools_json = serde_json::to_value(&body.tools)?;
        let res = sqlx::query(
            "UPDATE agents SET name = $1, provider = $2, model = $3, \
                               system_prompt = $4, tools_json = $5, updated_at = $6 \
             WHERE id = $7",
        )
        .bind(&body.name)
        .bind(&body.provider)
        .bind(&body.model)
        .bind(&body.system_prompt)
        .bind(&tools_json)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        if res.rows_affected() == 0 {
            return Err(DomainError::NotFound(id.to_owned()));
        }
        self.get(id).await
    }

    pub async fn delete(&self, id: &str) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM agents WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(DomainError::NotFound(id.to_owned()));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct RunStore {
    pool: PgPool,
}

impl RunStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn record_started(&self, flow_id: &str) -> Result<Run, DomainError> {
        let id = crate::domain::new_id("run");
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO runs (id, flow_id, status, started_at) VALUES ($1, $2, 'running', $3)",
        )
        .bind(&id)
        .bind(flow_id)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(Run {
            id,
            flow_id: flow_id.to_owned(),
            status: "running".into(),
            started_at: now,
            finished_at: None,
            trace: None,
        })
    }

    pub async fn record_finished(
        &self,
        run_id: &str,
        status: &str,
        trace: Option<&serde_json::Value>,
    ) -> Result<(), DomainError> {
        let now = Utc::now();
        sqlx::query(
            "UPDATE runs SET status = $1, finished_at = $2, trace_json = $3 WHERE id = $4",
        )
        .bind(status)
        .bind(now)
        .bind(trace)
        .bind(run_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_for_flow(&self, flow_id: &str) -> Result<Vec<Run>, DomainError> {
        let rows: Vec<RunRow> = sqlx::query_as(
            "SELECT id, flow_id, status, started_at, finished_at, trace_json \
             FROM runs WHERE flow_id = $1 ORDER BY started_at DESC LIMIT 50",
        )
        .bind(flow_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(
                |(id, flow_id, status, started_at, finished_at, trace_json)| {
                    Ok(Run {
                        id,
                        flow_id,
                        status,
                        started_at,
                        finished_at,
                        trace: trace_json
                            .map(|v| serde_json::from_value(v))
                            .transpose()?,
                    })
                },
            )
            .collect()
    }
}

fn empty_graph() -> serde_json::Value {
    serde_json::json!({ "nodes": [], "edges": [] })
}

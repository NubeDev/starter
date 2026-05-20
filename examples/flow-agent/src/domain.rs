//! Domain types for flow-agent.
//!
//! `FlowGraph` is stored as opaque JSON in SQLite (column `graph_json`);
//! the runtime knows the shape because `@nube/starter-ui-flow`'s
//! `FlowGraph` and `starter-flow-spi::FlowTopology` agree on it. We
//! deliberately don't enforce the shape at the REST boundary — saving
//! a draft graph with dangling edges must be allowed; only `/fire`
//! validates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ---------------------------------------------------------------------
// Flows
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlowSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Flow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// Opaque graph blob (matches `starter-ui-flow`'s `FlowGraph` JSON).
    pub graph: serde_json::Value,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateFlow {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Optional initial graph. If omitted, an empty graph is stored.
    #[serde(default)]
    pub graph: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateFlow {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub graph: serde_json::Value,
    /// Optimistic lock — must match the row's current `version`.
    pub version: i64,
}

// ---------------------------------------------------------------------
// Agents
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentSummary {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    /// Reverse-DNS tool ids the agent may call. `flow:<id>` entries bind
    /// to flow runs through the agent-as-tool bridge (Phase 5).
    pub tools: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAgent {
    pub name: String,
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAgent {
    pub name: String,
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
}

// ---------------------------------------------------------------------
// Runs
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Run {
    pub id: String,
    pub flow_id: String,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub trace: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct FirePayload {
    /// Optional named trigger. The MVP only supports a single explicit
    /// trigger per flow ("default"); a future revision plumbs named
    /// triggers through to the engine.
    #[serde(default)]
    pub trigger: Option<String>,
    /// Free-form JSON sent to the trigger node's `payload` slot.
    #[serde(default = "serde_json::Value::default")]
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FireResponse {
    pub run_id: String,
}

// ---------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DomainError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("optimistic-lock conflict: version mismatch")]
    VersionConflict,
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", uuid::Uuid::new_v4().simple())
}

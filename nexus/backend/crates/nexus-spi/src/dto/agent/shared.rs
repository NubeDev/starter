//! Types shared across the agent verbs.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

/// A configured agent in full. `config` is opaque JSON on the wire — the
/// provider/agent-specific knobs the nexus-ai facade interprets at run time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AgentDetail {
    pub id: Uuid,
    pub name: String,
    /// The facade backend: an inference provider hint or a coding-agent backend.
    pub backend: String,
    /// Concrete model id or a size alias ("small"/"medium"/"large").
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    pub config: Value,
}

/// One session against an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SessionDetail {
    pub id: Uuid,
    pub agent_id: Uuid,
    /// Lifecycle: pending | running | completed | failed | cancelled.
    pub status: String,
    /// The message transcript: an array of `{role, content}` objects.
    pub transcript: Value,
}

//! Row and input shapes for the agents store.

use serde_json::Value;
use uuid::Uuid;

/// A saved agent configuration as stored. `config` is opaque here — the nexus-ai
/// facade interprets it when a session runs, the store only persists it.
#[derive(Debug, Clone)]
pub struct AgentRecord {
    pub id: Uuid,
    pub tenant_id: String,
    pub name: String,
    pub backend: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub config: Value,
}

/// A new agent to insert.
#[derive(Debug, Clone)]
pub struct NewAgent {
    pub name: String,
    pub backend: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub config: Value,
}

/// A partial update; `None` fields are left unchanged. `system_prompt` uses a
/// nested `Option` so the caller can distinguish "leave unchanged" (`None`) from
/// "clear it" (`Some(None)`).
#[derive(Debug, Clone, Default)]
pub struct AgentPatch {
    pub name: Option<String>,
    pub backend: Option<String>,
    pub model: Option<String>,
    pub system_prompt: Option<Option<String>>,
    pub config: Option<Value>,
}

/// A session against an agent as stored.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: Uuid,
    pub tenant_id: String,
    pub agent_id: Uuid,
    pub status: String,
    /// The message transcript: a JSON array of `{role, content}` objects.
    pub transcript: Value,
}

/// A new session to insert. Starts `pending` with the opening transcript.
#[derive(Debug, Clone)]
pub struct NewSession {
    pub agent_id: Uuid,
    pub transcript: Value,
}

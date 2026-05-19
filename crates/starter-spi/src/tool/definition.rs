//! The static, serializable description of a tool — what an MCP
//! client lists. The behaviour lives in [`super::Tool`].

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Tool metadata. Carries the JSON-schema for inputs so the
/// transport layer can validate before invoking.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ToolDefinition {
    /// Stable identifier. Snake-case, namespaced by domain
    /// (`gh.list_repos`, `iot.read_telemetry`).
    pub name: String,

    /// One-sentence human description. The LLM reads this.
    pub description: String,

    /// JSON-schema of the tool's input. The transport validates
    /// callers' input against this before invoking.
    pub input_schema: serde_json::Value,
}

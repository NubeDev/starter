//! `POST /api/v1/agents` request.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// Create an agent. `backend` selects the facade tier/provider, `model` is a
/// concrete id or size alias. `config` carries provider-specific knobs and is
/// validated when a session runs, not here. `model` defaults to `large`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreateAgentRequest {
    pub name: String,
    pub backend: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub config: Option<Value>,
}

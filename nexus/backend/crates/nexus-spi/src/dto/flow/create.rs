//! `POST /api/v1/flows` request.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// Create a flow. `input`/`pipeline`/`output` are the ArkFlow config blobs; the
/// FlowManager validates them when it builds the stream, so a malformed config
/// surfaces on start, not here. `enabled` defaults to false so a flow can be
/// created and reviewed before it runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreateFlowRequest {
    pub name: String,
    pub input: Value,
    #[serde(default)]
    pub pipeline: Option<Value>,
    pub output: Value,
    #[serde(default)]
    pub enabled: Option<bool>,
}

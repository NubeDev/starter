//! `PUT /api/v1/flows/:id` request.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// Partially update a flow; omitted fields are left unchanged. Toggling
/// `enabled` here does not by itself start/stop the flow — the dedicated
/// `start`/`stop` routes do that and also flip this flag — so a plain update is
/// for editing the config or name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UpdateFlowRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub input: Option<Value>,
    #[serde(default)]
    pub pipeline: Option<Value>,
    #[serde(default)]
    pub output: Option<Value>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

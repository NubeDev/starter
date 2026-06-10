//! `GET /api/v1/insights` — a stored insight in the tenant.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

/// One stored insight: immutable id, name, the script, and its optional params
/// schema. The flat list carries every field the client needs to run or edit it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct InsightSummary {
    pub id: Uuid,
    pub name: String,
    /// The Rhai transform script.
    pub script: String,
    /// Optional JSON-Schema for the script's params.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params_schema: Option<Value>,
}

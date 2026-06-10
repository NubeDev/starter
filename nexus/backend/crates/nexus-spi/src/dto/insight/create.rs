//! `POST /api/v1/insights` — create a stored insight.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// Create a tenant-scoped insight. `script` is the Rhai transform; `params_schema`
/// is an optional JSON-Schema describing the params a caller may pass when running
/// it (advisory metadata for the UI — the sandbox enforces safety regardless).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreateInsightRequest {
    /// Human-readable name, unique enough for the tenant's list.
    pub name: String,
    /// The Rhai script orchestrating the curated vectorized surface.
    pub script: String,
    /// Optional JSON-Schema for the script's params, for UI form generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params_schema: Option<Value>,
}

//! `PATCH /api/v1/insights/:id` — rename or rewrite a stored insight.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// Partial update. Any field left unset is unchanged. `params_schema` is updated
/// only when present — there is no separate "clear" since an absent schema and an
/// empty one are equivalent for the advisory UI use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UpdateInsightRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Replace the script.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// Replace the params schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params_schema: Option<Value>,
}

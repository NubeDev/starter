//! `POST /api/v1/dashboards/:slug/variables` — define a variable on a dashboard.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::shared::VariableKind;

/// Create a dashboard variable. `name` is unique within the dashboard; a
/// duplicate is a conflict. The kind-specific authoring input rides in
/// `options_config` (opaque, shape owned by the UI).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreateVariableRequest {
    /// Reference name without the `$`, unique per dashboard.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub kind: VariableKind,
    /// Kind-specific authoring config; defaults to an empty object.
    #[serde(default)]
    pub options_config: serde_json::Value,
    #[serde(default)]
    pub current: Vec<String>,
    #[serde(default)]
    pub multi: bool,
    #[serde(default)]
    pub include_all: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub sort_order: i32,
}

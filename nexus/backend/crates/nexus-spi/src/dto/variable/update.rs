//! `PATCH /api/v1/variables/:id` — edit a variable definition or its selection.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::shared::VariableKind;

/// Partial update. Every field is optional — `None` leaves the stored value
/// untouched. Changing `current` alone is the common case (the user picked a new
/// value in the bar); changing the rest is the variable editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UpdateVariableRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<VariableKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options_config: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multi: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_all: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
}

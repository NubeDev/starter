//! `PUT /api/v1/panels/:id` — update a panel.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

/// Partial update of a panel. `None` fields are left unchanged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UpdatePanelRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datasource_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viz: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<Value>,
    /// Three-valued (RW-06): omitted = leave the insight unchanged; `null` =
    /// detach the insight; an id = attach/replace it. The `Option<Option<_>>`
    /// encodes this — `None` (absent), `Some(None)` (explicit null → detach),
    /// `Some(Some(id))` (set). `#[serde(default)]` (not `skip_serializing_if`) so
    /// an explicit `null` deserializes to `Some(None)` rather than being elided.
    #[serde(default)]
    pub insight_id: Option<Option<Uuid>>,
    /// Three-valued like `insight_id`: omitted = unchanged, `null` = clear params,
    /// object = set params.
    #[serde(default)]
    pub insight_params: Option<Option<Value>>,
}

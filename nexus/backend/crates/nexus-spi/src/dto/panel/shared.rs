//! The panel view returned within a dashboard.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

/// A panel as the canvas renders it: which datasource + query feed it, how it is
/// drawn, and where it sits on the grid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PanelDetail {
    pub id: Uuid,
    pub title: String,
    /// Datasource the panel queries. `None` if its datasource was deleted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datasource_id: Option<Uuid>,
    /// The panel's SQL, run under the server query guards when refreshed.
    pub sql: String,
    /// Visualization kind: `line` | `bar` | `table` | `stat` | …
    pub viz: String,
    /// Opaque grid layout the canvas owns.
    pub layout: Value,
    /// Optional post-query insight (RW-06) applied to the panel's result. `None`
    /// = none attached, or the referenced insight was deleted (FK SET NULL).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insight_id: Option<Uuid>,
    /// Params bound as the insight script's `params`. `None` = no params.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insight_params: Option<Value>,
}

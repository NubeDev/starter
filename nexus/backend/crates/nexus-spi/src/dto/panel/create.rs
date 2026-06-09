//! `POST /api/v1/dashboards/:slug/panels` — add a panel to a dashboard.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

/// Create a panel under the dashboard named in the path. The dashboard is keyed
/// by its immutable id (resolved from the path slug at the request edge).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreatePanelRequest {
    pub title: String,
    /// Datasource this panel queries.
    pub datasource_id: Uuid,
    pub sql: String,
    /// Visualization kind; defaults to `table` when omitted.
    #[serde(default)]
    pub viz: Option<String>,
    /// Grid layout; defaults to empty when omitted.
    #[serde(default)]
    pub layout: Option<Value>,
}

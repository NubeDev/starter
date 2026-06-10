//! `GET /api/v1/dashboards` — list dashboards for the tenant.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// A dashboard in a list: identity, route alias, display name, and appearance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DashboardSummary {
    /// Immutable id — grants and panel refs key on this.
    pub id: Uuid,
    /// Mutable route alias.
    pub slug: String,
    pub name: String,
    /// lucide icon name for the sidebar/page chrome.
    pub icon: String,
    /// accent colour as an HSL triple string, e.g. "152 76% 44%".
    pub accent: String,
    /// Folder this dashboard is filed under (WS-05); `null` is the root.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<Uuid>,
    /// Whether the dashboard is starred (WS-05).
    #[serde(default)]
    pub starred: bool,
}

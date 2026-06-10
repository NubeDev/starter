//! `POST /api/v1/dashboards` — create a dashboard.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Create a dashboard. The slug must be unique within the tenant; the server
/// rejects a duplicate rather than silently aliasing. Appearance (icon +
/// accent) is optional — omitted fields fall back to server defaults, so older
/// clients that send only name/slug keep working.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreateDashboardRequest {
    pub slug: String,
    pub name: String,
    /// lucide icon name; defaults server-side when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// accent colour as an HSL triple string, e.g. "152 76% 44%"; defaults
    /// server-side when omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    /// Folder to file the dashboard under (WS-05); omit/`null` for the root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<Uuid>,
}

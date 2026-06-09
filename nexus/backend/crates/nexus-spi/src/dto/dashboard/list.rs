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
}

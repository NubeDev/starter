//! `PUT /api/v1/dashboards/:slug` — rename or re-slug a dashboard.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Partial update. Renaming the slug changes only the route alias — grants and
/// panel refs keep pointing at the immutable id, so nothing is orphaned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct UpdateDashboardRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// lucide icon name; `None` leaves it unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// accent HSL triple string; `None` leaves it unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
}

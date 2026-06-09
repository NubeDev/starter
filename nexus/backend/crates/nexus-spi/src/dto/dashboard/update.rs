//! `PUT /api/v1/dashboards/:slug` — rename or re-slug a dashboard.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Partial update. Renaming the slug changes only the route alias — grants and
/// panel refs keep pointing at the immutable id, so nothing is orphaned. Moving
/// between folders and starring (WS-05) ride this same patch.
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
    /// Move into this folder. Ignored when `clear_folder` is true; `None` (with
    /// `clear_folder` false) leaves the folder unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<Uuid>,
    /// Re-root the dashboard (clear its folder).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub clear_folder: bool,
    /// Star/unstar; `None` leaves it unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starred: Option<bool>,
}

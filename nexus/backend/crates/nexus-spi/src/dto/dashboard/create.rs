//! `POST /api/v1/dashboards` — create a dashboard.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Create a dashboard. The slug must be unique within the tenant; the server
/// rejects a duplicate rather than silently aliasing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreateDashboardRequest {
    pub slug: String,
    pub name: String,
}

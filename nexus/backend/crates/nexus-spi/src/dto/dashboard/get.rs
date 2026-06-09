//! `GET /api/v1/dashboards/:slug` — a dashboard with its panels.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::dto::panel::PanelDetail;

/// Full dashboard view: identity plus its ordered panels. The slug resolves to
/// the `id` at the request edge; everything below keys on the id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DashboardDetail {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    /// lucide icon name for the sidebar/page chrome.
    pub icon: String,
    /// accent colour as an HSL triple string, e.g. "152 76% 44%".
    pub accent: String,
    pub panels: Vec<PanelDetail>,
}

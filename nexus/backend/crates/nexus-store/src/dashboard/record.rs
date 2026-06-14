//! Dashboard and panel records and their create/update inputs.

use serde_json::Value;
use uuid::Uuid;

/// A stored dashboard.
#[derive(Debug, Clone)]
pub struct DashboardRecord {
    pub id: Uuid,
    pub tenant_id: String,
    pub slug: String,
    pub name: String,
    /// lucide icon name for the sidebar/page chrome.
    pub icon: String,
    /// accent colour as an HSL triple string, e.g. "152 76% 44%".
    pub accent: String,
    /// Folder this dashboard is filed under; `None` is the root (WS-05).
    pub folder_id: Option<Uuid>,
    /// Whether the caller's tenant has starred this dashboard (WS-05).
    pub starred: bool,
}

/// A stored panel.
#[derive(Debug, Clone)]
pub struct PanelRecord {
    pub id: Uuid,
    pub dashboard_id: Uuid,
    pub datasource_id: Option<Uuid>,
    pub title: String,
    pub sql: String,
    pub viz: String,
    pub layout: Value,
    /// Optional post-query insight (RW-06) applied to this panel's result before
    /// it reaches the widget. `None` = the raw query result is rendered.
    pub insight_id: Option<Uuid>,
    /// JSON bound as the insight script's `params`. `None` = no params.
    pub insight_params: Option<Value>,
}

/// Input to create a dashboard.
#[derive(Debug, Clone)]
pub struct NewDashboard {
    pub slug: String,
    pub name: String,
    pub icon: String,
    pub accent: String,
    /// Folder to file the new dashboard under; `None` is the root (WS-05).
    pub folder_id: Option<Uuid>,
}

/// Partial update of a dashboard. Every field is optional — `None` leaves the
/// current value untouched (COALESCE in the store). The immutable id is not
/// patchable; re-slugging changes only the route alias.
#[derive(Debug, Clone, Default)]
pub struct DashboardPatch {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub icon: Option<String>,
    pub accent: Option<String>,
    /// Move to a folder (`Some(Some(id))`), re-root (`Some(None)`), or leave
    /// unchanged (`None`) — the three-valued case COALESCE can't express (WS-05).
    pub folder_id: Option<Option<Uuid>>,
    /// Star/unstar; `None` leaves it unchanged.
    pub starred: Option<bool>,
}

/// Input to create a panel under a dashboard.
#[derive(Debug, Clone)]
pub struct NewPanel {
    pub dashboard_id: Uuid,
    pub datasource_id: Option<Uuid>,
    pub title: String,
    pub sql: String,
    pub viz: String,
    pub layout: Value,
    /// Optional post-query insight (RW-06); `None` = none attached at create.
    pub insight_id: Option<Uuid>,
    /// Params bound as the insight script's `params`; `None` = no params.
    pub insight_params: Option<Value>,
}

/// Partial update of a panel. Every field is optional — `None` leaves the
/// current value untouched (COALESCE in the store). The panel's `dashboard_id`
/// is not patchable.
#[derive(Debug, Clone, Default)]
pub struct PanelPatch {
    pub title: Option<String>,
    pub datasource_id: Option<Uuid>,
    pub sql: Option<String>,
    pub viz: Option<String>,
    pub layout: Option<Value>,
    /// Attach (`Some(Some(id))`), detach (`Some(None)`), or leave unchanged
    /// (`None`) the panel's insight. Three-valued because COALESCE can't express
    /// "set to NULL" — detaching an insight from a panel is a real edit (the user
    /// removes it), unlike datasource which the UI only ever sets. See the
    /// `folder_id` field on `DashboardPatch` for the same pattern.
    pub insight_id: Option<Option<Uuid>>,
    /// Set (`Some(Some(json))`), clear (`Some(None)`), or leave unchanged
    /// (`None`) the insight params. Tracks `insight_id` so detaching clears both.
    pub insight_params: Option<Option<Value>>,
}

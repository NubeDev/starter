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
}

/// Input to create a dashboard.
#[derive(Debug, Clone)]
pub struct NewDashboard {
    pub slug: String,
    pub name: String,
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
}

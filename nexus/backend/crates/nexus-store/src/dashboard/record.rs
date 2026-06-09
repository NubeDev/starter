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

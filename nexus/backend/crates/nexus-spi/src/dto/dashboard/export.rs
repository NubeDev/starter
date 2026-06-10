//! The portable dashboard JSON model (WS-05, contract C1).
//!
//! `GET /api/v1/dashboards/:slug/export` emits a [`DashboardExport`]; `POST
//! /api/v1/dashboards/import` validates `schema_version` and re-creates from one.
//! The shape is self-contained — appearance, panels, and variables travel
//! together — so an exported dashboard is portable across tenants and is the seam
//! the AI "Ask Nexus" generator emits into. Datasource ids are carried as-is; an
//! import into a tenant that lacks a referenced datasource leaves that panel's
//! datasource unset rather than failing the whole import.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

/// The current dashboard-model schema version. Bumped only on a breaking change;
/// an import rejects a version it does not understand.
pub const DASHBOARD_SCHEMA_VERSION: u32 = 1;

/// A self-contained, importable dashboard. Identity (`slug`/`name`) plus
/// appearance, its panels, and its variables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DashboardExport {
    /// Model version; must match [`DASHBOARD_SCHEMA_VERSION`] to import.
    pub schema_version: u32,
    pub slug: String,
    pub name: String,
    /// lucide icon name.
    pub icon: String,
    /// accent HSL triple string.
    pub accent: String,
    pub panels: Vec<PanelExport>,
    #[serde(default)]
    pub variables: Vec<VariableExport>,
}

/// One panel in the export — everything needed to re-create it bar the ids the
/// importing tenant mints fresh.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct PanelExport {
    pub title: String,
    /// Datasource this panel queries; `null` when the source dashboard had none
    /// or the importing tenant should re-bind it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datasource_id: Option<Uuid>,
    pub sql: String,
    pub viz: String,
    pub layout: Value,
}

/// One variable in the export. Mirrors the relational variable row minus its
/// ids; opaque `options_config` travels as-is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct VariableExport {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub kind: String,
    #[serde(default)]
    pub options_config: Value,
    #[serde(default)]
    pub current: Vec<String>,
    #[serde(default)]
    pub multi: bool,
    #[serde(default)]
    pub include_all: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub sort_order: i32,
}

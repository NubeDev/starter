//! Dashboard-variable records and their create/update inputs.
//!
//! The store keeps the variable's kind as a plain string (its wire `snake_case`
//! form) — the DTO layer owns the enum, the store stays schema-shaped.

use serde_json::Value;
use uuid::Uuid;

/// A stored dashboard variable.
#[derive(Debug, Clone)]
pub struct VariableRecord {
    pub id: Uuid,
    pub dashboard_id: Uuid,
    pub name: String,
    pub label: Option<String>,
    /// Kind as its wire string (`constant`/`custom`/`query`/…).
    pub kind: String,
    /// Kind-specific authoring config, opaque to the store.
    pub options_config: Value,
    /// Selected value(s) as a JSON array of strings.
    pub current: Vec<String>,
    pub multi: bool,
    pub include_all: bool,
    pub hidden: bool,
    pub sort_order: i32,
}

/// Input to create a variable under a dashboard.
#[derive(Debug, Clone)]
pub struct NewVariable {
    pub dashboard_id: Uuid,
    pub name: String,
    pub label: Option<String>,
    pub kind: String,
    pub options_config: Value,
    pub current: Vec<String>,
    pub multi: bool,
    pub include_all: bool,
    pub hidden: bool,
    pub sort_order: i32,
}

/// Partial update of a variable. `None` leaves the stored value untouched
/// (COALESCE in the store). The dashboard scope and the immutable id are not
/// patchable.
#[derive(Debug, Clone, Default)]
pub struct VariablePatch {
    pub name: Option<String>,
    pub label: Option<String>,
    pub kind: Option<String>,
    pub options_config: Option<Value>,
    pub current: Option<Vec<String>>,
    pub multi: Option<bool>,
    pub include_all: Option<bool>,
    pub hidden: Option<bool>,
    pub sort_order: Option<i32>,
}

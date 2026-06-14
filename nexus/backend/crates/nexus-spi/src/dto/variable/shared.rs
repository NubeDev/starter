//! The variable record + its kind, shared by the create/update verbs.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// The kind of a dashboard variable — what populates its option list and how its
/// value is sourced. Mirrors Grafana's templating types; the UI resolves the
/// options for each kind, the binder only ever sees the *resolved* string
/// value(s) (WS-03's `$var`/`$__sqlIn` expansion).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VariableKind {
    /// A fixed value, usually hidden — a named constant interpolated into SQL.
    Constant,
    /// A static comma-separated list of options authored on the variable.
    Custom,
    /// Options come from running the variable's SQL against a datasource; the
    /// first column of each row is an option. Subject to the same query guards
    /// as a panel query.
    Query,
    /// Options are the tenant's datasources (optionally of one kind), so a
    /// `$ds` variable can drive which datasource panels query.
    Datasource,
    /// A list of durations (e.g. `1m,5m,1h`) that can drive `$__interval`.
    Interval,
    /// A free-text box — the value is whatever the user types.
    Textbox,
    /// Value is read from the page's context (WS-13): the nav node, the URL, the
    /// dashboard's tags, or the nav node's `values` override. `options_config`
    /// carries `{ source: 'nav'|'url'|'tag'|'values', key }`; resolution is
    /// synchronous (no fetch) — the UI assembles the `PageContext` and reads it.
    /// The binder still only sees the resolved string value, like any kind.
    Context,
}

/// A dashboard variable definition. `options_config` carries the kind-specific
/// authoring input (the custom list, the option SQL + datasource, the interval
/// steps, or the datasource-kind filter); it is opaque on the wire because its
/// shape varies by `kind` and the UI owns each shape. `current` holds the
/// selected value(s); the binder receives these (resolved) as `QueryVariable`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct VariableDetail {
    pub id: Uuid,
    /// The dashboard this variable scopes.
    pub dashboard_id: Uuid,
    /// The reference name without the `$` (e.g. `region`). Unique per dashboard.
    pub name: String,
    /// Human label for the variable bar; falls back to `name` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub kind: VariableKind,
    /// Kind-specific authoring config (opaque; shape owned by the UI per kind).
    pub options_config: serde_json::Value,
    /// Currently selected value(s). One entry for a single-select; several for a
    /// multi-select / "All" expansion. The binder binds each as its own arg.
    #[serde(default)]
    pub current: Vec<String>,
    /// Whether multiple values may be selected at once.
    #[serde(default)]
    pub multi: bool,
    /// Whether an "All" option is offered (expands to every resolved option).
    #[serde(default)]
    pub include_all: bool,
    /// Whether the variable is hidden from the bar (constants usually are).
    #[serde(default)]
    pub hidden: bool,
    /// Display/resolution order within the dashboard's variable bar; lower
    /// sorts first. Ties break by creation order.
    #[serde(default)]
    pub sort_order: i32,
}

//! Server-Driven UI — Component IR.
//!
//! Ported from `rubix-contracts/ui-ir` (see
//! `DOCS/frontend/sdui/SCOPE.md` and `DOCS/frontend/sdui/DIVERGENCE.md`).
//! This crate is the zero-I/O substrate for starter's SDUI: types, the
//! [`Bindable`] trait, chart source / kind newtypes, and the
//! [`IR_VERSION`] stamp. Per R1 the only dependencies are
//! `serde`, `serde_json`, `schemars`, `thiserror`, and `tracing` — no
//! axum / reqwest / tokio / tower transitively. A `cargo tree
//! --edges normal` denylist gate in CI enforces that.
//!
//! Every component is a variant of [`Component`] discriminated by a
//! stable `"type"` field on the wire.
//!
//! Divergence from Rubix lives in `DOCS/frontend/sdui/DIVERGENCE.md`.
//! Two drifts apply at this layer:
//!
//! - **D1** — the action-response variant Rubix called `form_errors`
//!   ships here as [`ActionResponse::Diagnostics`] only. Starter has
//!   not shipped; the `form_errors` tag is rejected at the wire.
//! - **D3** — the Rust side is split into three narrow crates
//!   (`starter-ui-ir`, `starter-ui-bindings`, `starter-ui-builder`).
//!   This crate is the bottom of that stack.

mod action;
mod bindable;
mod chart;
mod component;
mod diagnostic;
pub mod schema;

pub use action::{ActionContext, ActionRequest, ActionResponse, NavigateTo, ToastIntent};
pub use bindable::{expected_shape, json_shape, Bindable, ResolveIssue};
pub use diagnostic::{Diagnostic, Severity};

pub use chart::{
    AggSpec, AnalyticsTemplateMap, ChartHistory, ChartHistoryPreset, ChartKind, ChartRange,
    ChartSeries, ChartSource, DataPoint, OrderDirection, RowsOrder,
};
pub use component::{
    Action, BindingSpec, Bindings, ColumnRender, Component, Concurrency, ConfirmDialog,
    DateRangePreset, DiffAnnotation, FieldError, FlexAlign, FlexJustify, JsonTableColumn,
    JsonTableColumnFormat, KpiDelta, KpiGridItem, NodeStyle, NumberValidate, OptimisticHint,
    RowAction, RowBreakpoints, RowLayout, SelectOption, ShowWhen, Tab, TableColumn, TableSource,
    TextValidate, TimelineEvent, ToolbarAction, TreeItem,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Serde helper — skip serializing a `bool` field when it is `false`.
fn is_false(b: &bool) -> bool {
    !*b
}

/// Serde helper — skip serializing a `bool` field when it is `true`.
/// Pairs with [`default_true`] for `default-true` flags whose serialised
/// form is the more interesting deviation from the default.
pub(crate) fn is_true(b: &bool) -> bool {
    *b
}

/// Serde helper — default value for `default-true` boolean fields.
pub(crate) fn default_true() -> bool {
    true
}

/// IR version stamped at the root of every tree. The client advertises
/// supported versions in the capability handshake; the server clamps
/// emission to the highest mutually-supported version. Adding a
/// component variant is a minor bump; removing or re-shaping is a
/// major bump with a 12-month deprecation window.
///
/// v2: added `toggle`, `slider` variants; `BindingSpec`, `Concurrency` types.
///
/// v3: reshaped `ChartSource` from a flat `{node_id, slot, field}` struct
/// into a discriminated enum (`series` | `series_by_kind` | `rows` |
/// `series_from_rsql` | `static`); added `ChartKind` and `AggSpec`
/// enums; `Component::Chart` now carries `sources: Vec<ChartSource>`
/// (was `source: ChartSource`) plus a typed `kind` and an optional
/// page-level `agg`. The resolver downgrades V3-only chart variants to
/// [`Component::Dangling`] when the client sends `Accept: …; v=2`.
///
/// v4: added `Component::Table.row_actions` (per-row buttons),
/// `Component::Table.toolbar_actions` (page-level buttons above the
/// table), and the `RowAction` / `ToolbarAction` / `ConfirmDialog`
/// structs. Both new fields default to empty vec, so V3 trees still
/// deserialise; V3 clients silently lose the buttons (additive
/// change).
///
/// v5: added `Component::Divider` (V1.1 of SDUI-VISUAL.md). Carries
/// `orientation`, `intent`, and `spacing` tokens — never pixel
/// values. Additive: V4 trees still deserialise; V4 clients
/// downgrade unknown variants via the `Dangling` fallback or refuse
/// the tree at the capability handshake.
pub const IR_VERSION: u32 = 5;

/// Names of variants in the portable IR subset — those a non-web
/// renderer (react-native, SwiftUI, Flutter) can implement without
/// DOM/CSS assumptions. The runtime equivalent of
/// [`Component::is_portable`]; kept in lock-step with that match.
///
/// Variants NOT in this list either embed CSS-length fields, use
/// HTML elements as their rendering contract, or carry verbatim
/// platform-specific props. Non-web renderers must implement them
/// with a documented platform mapping or downgrade to `Dangling` via
/// the capability handshake.
pub const IR_PORTABLE_VARIANTS: &[&str] = &[
    "page",
    "row",
    "col",
    "grid",
    "card",
    "tabs",
    "divider",
    "text",
    "kpi",
    "chart",
    "table",
    "form",
    "select",
    "toggle",
    "slider",
    "date_range",
    "ref_picker",
    "repeat",
];

/// Root of every component tree. Carries the IR version so clients can
/// refuse to render incompatible trees.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComponentTree {
    /// Protocol version — currently [`IR_VERSION`].
    pub ir_version: u32,
    /// The root component (always a `page` variant for resolve output).
    pub root: Component,
    /// Author-declared constants, referenced from bindings via
    /// `{{$vars.<key>}}`. Scoped to the whole tree; resolved once per
    /// resolve call, before any other binding substitution. Values are
    /// any JSON — strings, numbers, arrays, nested objects. Vars
    /// cannot reference other vars in v1 (no recursion).
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub vars: std::collections::HashMap<String, serde_json::Value>,
}

impl ComponentTree {
    /// Build a tree with the current [`IR_VERSION`].
    pub fn new(root: Component) -> Self {
        Self {
            ir_version: IR_VERSION,
            root,
            vars: std::collections::HashMap::new(),
        }
    }
}

// `bindable_derive_smoke` from Rubix exercised the `#[derive(Bindable)]`
// proc-macro. Starter does not ship that derive (R1 forbids proc-macro
// deps on `starter-ui-ir`), so `Component`'s `Bindable` impl is the
// hand-written dispatch in `component.rs`. The trait contract is
// covered by the per-variant tests in that module's `tests` submodule
// and by the integration test in `tests/bindable_dispatch.rs`.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_minimal_tree() {
        let tree = ComponentTree::new(Component::Page {
            id: "p1".into(),
            title: Some("Hello".into()),
            header_actions: vec![],
            children: vec![],
            style: None,
            default_row_gap: None,
            default_column_gap: None,
            default_page_padding: None,
            default_max_width: None,
        });
        let json = serde_json::to_value(&tree).unwrap();
        assert_eq!(json["ir_version"], IR_VERSION);
        assert_eq!(json["root"]["type"], "page");
        assert_eq!(json["root"]["title"], "Hello");

        let back: ComponentTree = serde_json::from_value(json).unwrap();
        assert_eq!(back.ir_version, IR_VERSION);
    }

    #[test]
    fn round_trip_nested_tree() {
        let tree = ComponentTree::new(Component::Page {
            id: "p1".into(),
            title: Some("Test".into()),
            header_actions: vec![],
            default_row_gap: None,
            default_column_gap: None,
            default_page_padding: None,
            default_max_width: None,
            children: vec![Component::Col {
                id: None,
                span: None,
                align: None,
                justify: None,
                children: vec![
                    Component::Text {
                        id: Some("t1".into()),
                        content: "Hello".into(),
                        intent: None,
                        style: None,
                    },
                    Component::Button {
                        id: Some("b1".into()),
                        label: "Click".into(),
                        intent: None,
                        disabled: None,
                        action: Some(Action {
                            handler: "do_thing".into(),
                            args: None,
                            optimistic: None,
                        }),
                        style: None,
                    },
                ],
                gap: None,
                style: None,
            }],
            style: None,
        });
        let json = serde_json::to_string(&tree).unwrap();
        let back: ComponentTree = serde_json::from_str(&json).unwrap();
        match &back.root {
            Component::Page { children, .. } => assert_eq!(children.len(), 1),
            other => panic!("expected Page, got {other:?}"),
        }
    }

    #[test]
    fn action_widget_round_trips() {
        let widget = Component::ActionWidget {
            id: Some("w1".into()),
            action_ref: "com.acme.weather.fetch".into(),
            target: "/devices/weather-1".into(),
            title: Some("Get Weather".into()),
            description: None,
        };
        let json = serde_json::to_value(&widget).unwrap();
        assert_eq!(json["type"], "action_widget");
        assert_eq!(json["action_ref"], "com.acme.weather.fetch");
        assert_eq!(json["target"], "/devices/weather-1");
        assert_eq!(json["title"], "Get Weather");
        assert!(
            json.get("description").is_none(),
            "absent fields elided: {json}"
        );
        let _back: Component = serde_json::from_value(json).unwrap();
    }

    #[test]
    fn json_schema_emits() {
        let schema = schemars::schema_for!(ComponentTree);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(json.contains("ComponentTree"));
        assert!(json.contains("ir_version"));
    }
}

//! Server-Driven UI — typed Rust builder DSL for authoring
//! `ComponentTree`s from `main.rs`.
//!
//! Phase 3 of the SDUI port (DOCS/frontend/sdui/SCOPE.md). Ported from
//! `extension-sdk/sdui-builder` in the Rubix workspace. Starter owns
//! the ported copy going forward.
//!
//! # The compile-time / resolve-time contract
//!
//! Two classes of mistake exist for an authored page; the builder
//! catches one at compile time and defers the other to resolve time.
//! From SCOPE.md § Surface — Rust (builder DSL):
//!
//! > **What is compile-time-checked, and what isn't.** The builder
//! > uses newtype source/kind pairing (Rubix's `TimeSeriesSource` /
//! > `RowsSource` pattern): passing a `RowsSource` to `line_chart` is
//! > a build error. Component-level shape errors (a `kpi` without a
//! > `value`) are also compile-time, by the type system.
//! >
//! > **Binding strings are not compile-time-checked.** A typo in
//! > `target("outdoor-temp")` produces a valid `String` that the
//! > binding engine will fail to resolve at request time, returning a
//! > structured `BindingError`. Validating bindings at compile time
//! > would require `starter-ui-builder` to depend on
//! > `starter-ui-bindings` (so it could parse the grammar) and on a
//! > per-consumer `EntityGraph` shape (so it could verify the child /
//! > slot exists) — the first would couple two crates the dependency
//! > split is built to keep separate; the second is impossible
//! > without consumer-specific generics. The trade is intentional:
//! > source/kind get compile-time safety, binding strings get
//! > resolve-time errors with line-numbered diagnostics.
//! >
//! > The dependency split holds: `starter-ui-builder` depends only
//! > on `starter-ui-ir` (for the types it constructs), not on
//! > `starter-ui-bindings`. A consumer authoring pages-as-code from
//! > `main.rs` pulls `ir + builder`; the binding engine ships on the
//! > server.
//!
//! # Worked example
//!
//! ```
//! use starter_ui_builder::prelude::*;
//!
//! let tree = dashboard("overview", "{{$target.name}} Overview", [
//!     kpi_grid(
//!         "kpis",
//!         "1fr 1fr",
//!         [
//!             kpi("outdoor", "Outdoor Temp", series("outdoor-temp", "value")),
//!             kpi("energy",  "Energy (kWh)", series("kwh",          "value")),
//!         ],
//!     ),
//!     table(
//!         "alarms",
//!         rsql().kind("alarm.active"),
//!     )
//!     .live()
//!     .column("Time", "slots.ts.value")
//!     .column("Severity", "slots.severity.value")
//!     .build(),
//! ]);
//!
//! let v = serde_json::to_value(&tree).unwrap();
//! assert_eq!(v["root"]["type"], "page");
//! assert_eq!(v["root"]["children"][0]["type"], "grid");
//! ```

pub mod bindings;
pub mod charts;
pub mod dashboard;
pub mod data;
pub mod display;
pub mod forms;
pub mod inputs;
pub mod layout;
pub mod rsql;
pub mod seed;

pub mod prelude {
    //! One-stop import for authoring code.

    pub use crate::bindings::{page_state, self_, stack, target, user, vars};
    pub use crate::charts::{
        bar_chart, gauge, kpi, line_chart, rows, series, sparkline, RowsSource, TimeSeriesSource,
    };
    pub use crate::dashboard::{dashboard, kpi_grid};
    pub use crate::data::table;
    pub use crate::display::{badge, heading, text};
    pub use crate::forms::{action_form, form, ActionForm};
    pub use crate::inputs::{date_range, ref_picker, select, slider, toggle};
    pub use crate::layout::{card, col, grid, page, row, tabs};
    pub use crate::rsql::{rsql, RsqlBuilder};
    pub use crate::seed::{seed_page, PageStore, SeedError};
    pub use serde_json::{json, Value as JsonValue};
    pub use starter_ui_ir::{
        Action, ColumnRender, Component, ComponentTree, ConfirmDialog, RowAction, ToolbarAction,
        IR_VERSION,
    };
}

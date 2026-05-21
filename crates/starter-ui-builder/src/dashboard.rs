//! High-level composition helpers — `dashboard()` and `kpi_grid()`.
//!
//! These wrap the layout primitives in [`crate::layout`] with shapes
//! that recur across authored pages: a top-level page wrapping a
//! collection of cards, or a row / grid of KPI tiles.

use starter_ui_ir::{Component, ComponentTree, IR_VERSION};

use crate::layout::page;

/// Wrap a slice of children in a fresh [`ComponentTree`] rooted at a
/// [`Component::Page`]. Equivalent to:
///
/// ```ignore
/// ComponentTree {
///     ir_version: IR_VERSION,
///     root: page(id, title).children(kids).build(),
///     vars: Default::default(),
/// }
/// ```
///
/// but keeps authoring sites free of the boilerplate.
pub fn dashboard(
    id: impl Into<String>,
    title: impl Into<String>,
    children: impl IntoIterator<Item = Component>,
) -> ComponentTree {
    ComponentTree {
        ir_version: IR_VERSION,
        root: page(id, title).children(children).build(),
        vars: Default::default(),
    }
}

/// Build a [`Component::Grid`] holding KPI tiles. Each entry is
/// taken verbatim — call sites build the [`Component::Kpi`] via
/// [`crate::charts::kpi`] and pass the results in.
pub fn kpi_grid(
    id: impl Into<String>,
    columns: impl Into<String>,
    kpis: impl IntoIterator<Item = Component>,
) -> Component {
    use crate::layout::grid;
    grid(id).columns(columns).children(kpis).build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::charts::{kpi, series};
    use crate::display::heading;

    #[test]
    fn dashboard_wraps_children_in_page_root() {
        let tree = dashboard("p", "Title", [heading("Hello").build()]);
        assert_eq!(tree.ir_version, IR_VERSION);
        let v = serde_json::to_value(&tree).unwrap();
        assert_eq!(v["root"]["type"], "page");
        assert_eq!(v["root"]["title"], "Title");
        assert_eq!(v["root"]["children"][0]["type"], "heading");
    }

    #[test]
    fn kpi_grid_emits_grid_with_columns() {
        let g = kpi_grid(
            "kpis",
            "1fr 1fr",
            [
                kpi("a", "A", series("n1", "value")),
                kpi("b", "B", series("n2", "value")),
            ],
        );
        let v = serde_json::to_value(&g).unwrap();
        assert_eq!(v["type"], "grid");
        assert_eq!(v["columns"], "1fr 1fr");
        assert_eq!(v["children"].as_array().unwrap().len(), 2);
    }
}

//! Server-side DoS limits per SCOPE.md § R8.
//!
//! Each constant here pairs with one variant of
//! [`crate::WhatTag`]; tests in `tests/limits_413.rs` pin every
//! tag. Per SCOPE these are **starting points, not load-tested
//! numbers** — the first consumer that hits one of the
//! "inherited / unmeasured" rows is the signal to re-measure.

use serde_json::Value as JsonValue;
use starter_ui_ir::{Component, ComponentTree};

use crate::error::{SduiError, WhatTag};

/// `page_state` byte cap. R8: 64 KiB (inherited / unmeasured).
pub const MAX_PAGE_STATE_BYTES: usize = 64 * 1024;

/// Serialised render tree cap. R8: 2 MiB (reused — aligns with
/// `starter-server`'s existing render-tree cap).
pub const MAX_RENDER_TREE_BYTES: usize = 2 * 1024 * 1024;

/// Max IR tree nodes per resolve. R8: 2000 (inherited).
pub const MAX_TREE_NODES: usize = 2_000;

/// Max tree depth. R8: 32 (inherited; observed pages are ≤ 8).
pub const MAX_TREE_DEPTH: usize = 32;

/// Max distinct component variant types per page. R8: 60
/// (inherited — total vocabulary is ~30, leaves room for `custom`).
pub const MAX_COMPONENT_TYPES: usize = 60;

/// Max action handler timeout (server-side enforcement). R8: 5 s
/// (inherited). The server cancels the handler future at this
/// deadline and returns a `diagnostics` error; clients may give up
/// sooner but that is their policy.
pub const MAX_HANDLER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Max rows per table page. R8: 500 (inherited from Rubix S6
/// virtualisation testing).
pub const MAX_TABLE_ROWS_PER_PAGE: usize = 500;

// ---------------------------------------------------------------------------
// Enforcement helpers — each returns `Err(SduiError::PayloadTooLarge { what, .. })`
// on violation. The handler tests pin both the status and the tag.
// ---------------------------------------------------------------------------

/// Reject a `page_state` blob whose serialised size exceeds the
/// cap. Empty / null values short-circuit to 0 bytes.
pub fn enforce_page_state_bytes(page_state: &JsonValue) -> Result<(), SduiError> {
    let bytes = if page_state.is_null() {
        0
    } else {
        // Cheap upper-bound: serialising once is fine for a cap
        // measured in tens of KiB. If this ever shows up on a flame
        // graph, switch to a streaming counter.
        serde_json::to_vec(page_state).map(|v| v.len()).unwrap_or(0)
    };
    if bytes > MAX_PAGE_STATE_BYTES {
        return Err(SduiError::PayloadTooLarge {
            what: WhatTag::PageStateBytes,
            detail: format!(
                "page_state is {bytes} bytes, cap is {MAX_PAGE_STATE_BYTES}",
            ),
        });
    }
    Ok(())
}

/// Reject a serialised tree whose byte count exceeds the cap.
/// Counted on the bytes the client will receive — same as the
/// transport-level cap.
pub fn enforce_render_tree_bytes(serialised_len: usize) -> Result<(), SduiError> {
    if serialised_len > MAX_RENDER_TREE_BYTES {
        return Err(SduiError::PayloadTooLarge {
            what: WhatTag::RenderTreeBytes,
            detail: format!(
                "render tree is {serialised_len} bytes, cap is {MAX_RENDER_TREE_BYTES}",
            ),
        });
    }
    Ok(())
}

/// Walk a typed [`ComponentTree`] and enforce the node-count,
/// depth, and component-type caps in one pass.
pub fn enforce_tree_shape(tree: &ComponentTree) -> Result<(), SduiError> {
    let mut counter = ShapeCounter::default();
    counter.walk(&tree.root, 1)?;
    if counter.distinct_types.len() > MAX_COMPONENT_TYPES {
        return Err(SduiError::PayloadTooLarge {
            what: WhatTag::ComponentTypes,
            detail: format!(
                "{} distinct component types > cap {MAX_COMPONENT_TYPES}",
                counter.distinct_types.len(),
            ),
        });
    }
    Ok(())
}

/// Reject a tree whose raw JSON exceeds the depth limit **before**
/// it deserialises into [`ComponentTree`]. Deserialising a deeply
/// nested tree first would itself overflow the worker stack — the
/// JSON-shaped check fixes that the same way Rubix's
/// `enforce_layout_json_depth` does.
pub fn enforce_json_depth(raw: &JsonValue) -> Result<(), SduiError> {
    let depth = json_depth(raw, 0);
    if depth > MAX_TREE_DEPTH {
        return Err(SduiError::PayloadTooLarge {
            what: WhatTag::TreeDepth,
            detail: format!("layout depth {depth} > cap {MAX_TREE_DEPTH}"),
        });
    }
    Ok(())
}

fn json_depth(v: &JsonValue, current: usize) -> usize {
    match v {
        JsonValue::Object(map) => map
            .values()
            .map(|child| json_depth(child, current + 1))
            .max()
            .unwrap_or(current),
        JsonValue::Array(arr) => arr
            .iter()
            .map(|child| json_depth(child, current + 1))
            .max()
            .unwrap_or(current),
        _ => current,
    }
}

#[derive(Default)]
struct ShapeCounter {
    nodes: usize,
    distinct_types: std::collections::HashSet<&'static str>,
}

impl ShapeCounter {
    fn walk(&mut self, node: &Component, depth: usize) -> Result<(), SduiError> {
        self.nodes += 1;
        if self.nodes > MAX_TREE_NODES {
            return Err(SduiError::PayloadTooLarge {
                what: WhatTag::TreeNodes,
                detail: format!("tree exceeds {MAX_TREE_NODES} nodes"),
            });
        }
        if depth > MAX_TREE_DEPTH {
            return Err(SduiError::PayloadTooLarge {
                what: WhatTag::TreeDepth,
                detail: format!("tree depth {depth} > cap {MAX_TREE_DEPTH}"),
            });
        }
        self.distinct_types.insert(variant_tag(node));
        for child in children_of(node) {
            self.walk(child, depth + 1)?;
        }
        Ok(())
    }
}

fn variant_tag(c: &Component) -> &'static str {
    // Rough — names match the serde tag for the variants the
    // limits care about. Unknown / leaf variants fall through to a
    // generic tag, which still counts as one distinct type.
    match c {
        Component::Page { .. } => "page",
        Component::Row { .. } => "row",
        Component::Col { .. } => "col",
        Component::Grid { .. } => "grid",
        Component::Tabs { .. } => "tabs",
        Component::Card { .. } => "card",
        Component::Section { .. } => "section",
        Component::Text { .. } => "text",
        Component::Heading { .. } => "heading",
        Component::Badge { .. } => "badge",
        Component::Button { .. } => "button",
        Component::Table { .. } => "table",
        Component::Chart { .. } => "chart",
        Component::Kpi { .. } => "kpi",
        Component::KpiGrid { .. } => "kpi_grid",
        Component::Custom { .. } => "custom",
        Component::Dangling { .. } => "dangling",
        Component::Forbidden { .. } => "forbidden",
        Component::Divider { .. } => "divider",
        _ => "other",
    }
}

fn children_of(c: &Component) -> &[Component] {
    match c {
        Component::Page { children, .. }
        | Component::Row { children, .. }
        | Component::Col { children, .. }
        | Component::Grid { children, .. } => children,
        _ => &[],
    }
}

/// Reject a table query whose `size` parameter exceeds the cap.
pub fn enforce_table_page_size(size: usize) -> Result<(), SduiError> {
    if size > MAX_TABLE_ROWS_PER_PAGE {
        return Err(SduiError::PayloadTooLarge {
            what: WhatTag::TableRowsPerPage,
            detail: format!(
                "page size {size} > cap {MAX_TABLE_ROWS_PER_PAGE}",
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn page_state_bytes_under_cap_passes() {
        let v = json!({ "blob": "x".repeat(1024) });
        enforce_page_state_bytes(&v).unwrap();
    }

    #[test]
    fn page_state_bytes_over_cap_tags_correctly() {
        let v = json!({ "blob": "x".repeat(MAX_PAGE_STATE_BYTES + 1) });
        let err = enforce_page_state_bytes(&v).unwrap_err();
        match err {
            SduiError::PayloadTooLarge { what, .. } => {
                assert_eq!(what, WhatTag::PageStateBytes);
            }
            other => panic!("expected PayloadTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn json_depth_counts_nested_objects() {
        // Build {"a": {"a": {"a": ...}}} 40 deep
        let mut v = json!("leaf");
        for _ in 0..40 {
            v = json!({ "a": v });
        }
        let err = enforce_json_depth(&v).unwrap_err();
        match err {
            SduiError::PayloadTooLarge { what, .. } => {
                assert_eq!(what, WhatTag::TreeDepth);
            }
            other => panic!("expected TreeDepth, got {other:?}"),
        }
    }

    #[test]
    fn table_page_size_over_cap_tags_correctly() {
        let err = enforce_table_page_size(MAX_TABLE_ROWS_PER_PAGE + 1).unwrap_err();
        match err {
            SduiError::PayloadTooLarge { what, .. } => {
                assert_eq!(what, WhatTag::TableRowsPerPage);
            }
            other => panic!("expected TableRowsPerPage, got {other:?}"),
        }
    }
}

//! Sibling integration coverage for `RubixEntityGraph`.
//!
//! Covers the v1 surface from `rubix/docs/scope/dashboards/03-host-glue.md`:
//!
//! * `system` slots dispatch through the [`SystemSlotReader`] seam.
//! * Pool-less wiring on `flow:` / `user:` returns `None` (Phase B.2
//!   plugs the PG round-trip in behind a per-resolve cache).
//! * Unknown entity kinds return `None`.

use std::sync::Arc;

use rubix_agent::sdui::entity_graph::{RubixEntityGraph, StaticSystemReader, SystemSlotReader};
use serde_json::json;
use starter_ui_bindings::EntityGraph;

#[test]
fn system_slot_dispatches_through_reader() {
    let reader = Arc::new(StaticSystemReader::new().with("disk_percent", json!(91)))
        as Arc<dyn SystemSlotReader>;
    let graph = RubixEntityGraph::poolless(reader);
    assert_eq!(graph.read_slot("system", "disk_percent"), Some(json!(91)));
    assert_eq!(graph.read_slot("system", "unknown"), None);
}

#[test]
fn pool_less_flow_user_slots_return_none() {
    let graph = RubixEntityGraph::poolless(Arc::new(StaticSystemReader::new()));
    assert_eq!(graph.read_slot("flow:com.rubix.disk", "revision_id"), None);
    assert_eq!(graph.read_slot("user:op@example.com", "email"), None);
}

#[test]
fn unknown_entity_kind_returns_none() {
    let graph = RubixEntityGraph::poolless(Arc::new(StaticSystemReader::new()));
    assert_eq!(graph.read_slot("widget:1", "anything"), None);
    assert!(graph.read_children("system").is_empty());
}

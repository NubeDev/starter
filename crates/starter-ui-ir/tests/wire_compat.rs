//! Wire-shape compatibility tests for the IR version stamp and the
//! V5 chart dual-field tolerance the port carries forward.
//!
//! "Dual-field tolerance" follows Rubix's V5 pattern (SCOPE.md §R2):
//! a deserialise step accepts both the old and the new field name
//! for one release, default-writes the new, and ignores additive
//! fields a younger client doesn't know yet. The IR's V3 chart
//! reshape kept `ChartSource::Series` field names (`node_id`, `slot`,
//! `field`) so V2 wires remain readable, and the V5 stamp added
//! `Component::Divider` purely additively. This file pins those two
//! properties so a future contributor can't regress them quietly.

use serde_json::json;
use starter_ui_ir::{
    ChartKind, ChartSource, Component, ComponentTree, IR_VERSION,
};

#[test]
fn ir_version_stamp_is_v5() {
    // Stage 1 of the SDUI port lands at V5; bumping this requires a
    // matching SCOPE.md update (per R2 the bump is the contract).
    assert_eq!(IR_VERSION, 5);
    let tree = ComponentTree::new(Component::Page {
        id: "p".into(),
        title: None,
        header_actions: vec![],
        children: vec![],
        style: None,
        default_row_gap: None,
        default_column_gap: None,
        default_page_padding: None,
        default_max_width: None,
    });
    let v = serde_json::to_value(&tree).unwrap();
    assert_eq!(v["ir_version"], 5);
}

#[test]
fn v5_divider_round_trips_as_additive_variant() {
    // V5 added `Divider`; V4 deserialisers downgrade it via the
    // capability handshake. Locally we just verify the additive
    // wire shape stays intact.
    let raw = json!({
        "type": "divider",
        "orientation": "horizontal",
    });
    let c: Component = serde_json::from_value(raw).unwrap();
    let v = serde_json::to_value(&c).unwrap();
    assert_eq!(v["type"], "divider");
}

#[test]
fn chart_series_keeps_v2_field_names() {
    // Dual-field tolerance: the V3 `Series` variant declares the
    // exact field set V2 emitted (`node_id`, `slot`, `field`) so a
    // server emitting V3 trees stays readable by a V2 client that
    // ignores the new `"type": "series"` discriminator.
    let v = json!({
        "type": "series",
        "node_id": "abc",
        "slot": "out",
        "field": "payload.count"
    });
    let s: ChartSource = serde_json::from_value(v.clone()).unwrap();
    match &s {
        ChartSource::Series { node_id, slot, field } => {
            assert_eq!(node_id, "abc");
            assert_eq!(slot, "out");
            assert_eq!(field.as_deref(), Some("payload.count"));
        }
        other => panic!("expected Series, got {other:?}"),
    }
    // Round-trip: the on-wire shape is byte-stable for the V2-field
    // subset (the discriminator is the only addition).
    let back = serde_json::to_value(&s).unwrap();
    assert_eq!(back["node_id"], "abc");
    assert_eq!(back["slot"], "out");
    assert_eq!(back["field"], "payload.count");
}

#[test]
fn chart_source_unknown_type_is_forward_compat() {
    // A younger server may emit a source variant this client doesn't
    // know yet; the `#[serde(other)]` arm catches it instead of
    // failing the whole resolve.
    let v = json!({ "type": "future_tile_grid", "shards": 4 });
    let s: ChartSource = serde_json::from_value(v).unwrap();
    assert!(matches!(s, ChartSource::Unknown));
}

#[test]
fn chart_kind_unknown_string_round_trips_as_custom() {
    // Custom renderer kinds round-trip as the literal string — the
    // capability handshake (R7) filters unknown ids before emission;
    // here we just verify the wire grammar.
    let v = json!("acme.flow_canvas");
    let k: ChartKind = serde_json::from_value(v).unwrap();
    assert_eq!(k, ChartKind::Custom("acme.flow_canvas".into()));
}

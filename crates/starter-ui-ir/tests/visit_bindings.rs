//! Per-variant `Bindable::visit_bindings` smoke test.
//!
//! Each variant exercised here carries a `{{$target}}` token in one of
//! its string fields; the closure rewrites that exact substring to
//! `"X"`. The test asserts the rewrite landed — that's the bisect-
//! relevant property: the new trait method actually visits the field.

use starter_ui_ir::{Bindable, ChartSource, Component, KpiGridItem, SelectOption, Tab};

fn rewrite(s: &mut String) {
    *s = s.replace("{{$target}}", "X");
}

#[test]
fn visits_text_and_heading_content() {
    let mut t = Component::Text {
        id: None,
        content: "hello {{$target}}!".into(),
        intent: None,
        style: None,
    };
    t.visit_bindings(&mut rewrite);
    match t {
        Component::Text { content, .. } => assert_eq!(content, "hello X!"),
        _ => unreachable!(),
    }
}

#[test]
fn visits_badge_label_and_kpi_label() {
    let mut b = Component::Badge {
        id: None,
        label: "lvl-{{$target}}".into(),
        intent: None,
        style: None,
    };
    b.visit_bindings(&mut rewrite);
    if let Component::Badge { label, .. } = b {
        assert_eq!(label, "lvl-X");
    }

    let mut k = Component::Kpi {
        id: None,
        label: "k-{{$target}}".into(),
        source: ChartSource::Series {
            node_id: "n-{{$target}}".into(),
            slot: "value".into(),
            field: None,
        },
        value: None,
        format: None,
        intent: None,
        delta: None,
        unit_symbol: None,
        style: None,
    };
    k.visit_bindings(&mut rewrite);
    match k {
        Component::Kpi { label, source, .. } => {
            assert_eq!(label, "k-X");
            match source {
                ChartSource::Series { node_id, .. } => assert_eq!(node_id, "n-X"),
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }
}

#[test]
fn visits_chart_sources_and_tabs() {
    let mut c = Component::Chart {
        id: None,
        title: None,
        sources: vec![ChartSource::Series {
            node_id: "{{$target}}-id".into(),
            slot: "{{$target}}-slot".into(),
            field: Some("{{$target}}-f".into()),
        }],
        kind: Default::default(),
        agg: None,
        series: vec![],
        range: None,
        page_state_key: None,
        history: None,
    };
    c.visit_bindings(&mut rewrite);
    if let Component::Chart { sources, .. } = c {
        match &sources[0] {
            ChartSource::Series {
                node_id,
                slot,
                field,
            } => {
                assert_eq!(node_id, "X-id");
                assert_eq!(slot, "X-slot");
                assert_eq!(field.as_deref(), Some("X-f"));
            }
            _ => unreachable!(),
        }
    }

    let mut tabs = Component::Tabs {
        id: None,
        tabs: vec![Tab {
            id: Some("tab-{{$target}}".into()),
            label: "Tab {{$target}}".into(),
            icon: None,
            children: vec![],
        }],
        lazy: false,
        url_param: None,
        default: None,
    };
    tabs.visit_bindings(&mut rewrite);
    if let Component::Tabs { tabs, .. } = tabs {
        assert_eq!(tabs[0].label, "Tab X");
        assert_eq!(tabs[0].id.as_deref(), Some("tab-X"));
    }
}

#[test]
fn visits_select_options_and_kpi_grid_items() {
    let mut s = Component::Select {
        id: None,
        page_state_key: "k".into(),
        options: vec![SelectOption {
            label: "opt-{{$target}}".into(),
            value: serde_json::Value::Null,
        }],
        placeholder: None,
        default: None,
    };
    s.visit_bindings(&mut rewrite);
    if let Component::Select { options, .. } = s {
        assert_eq!(options[0].label, "opt-X");
    }

    let mut kg = Component::KpiGrid {
        id: None,
        columns: None,
        items: vec![KpiGridItem {
            id: None,
            label: "tile-{{$target}}".into(),
            value: serde_json::json!(0),
            format: None,
            intent: None,
            delta: None,
            on_click: None,
            unit_symbol: Some("{{$target}}u".into()),
        }],
        on_tile_click: None,
        style: None,
    };
    kg.visit_bindings(&mut rewrite);
    if let Component::KpiGrid { items, .. } = kg {
        assert_eq!(items[0].label, "tile-X");
        assert_eq!(items[0].unit_symbol.as_deref(), Some("Xu"));
    }
}

#[test]
fn layout_only_variants_are_noops() {
    // Row has no bindable string fields at this node level. The
    // visitor should be uncalled; we assert by passing a closure that
    // panics if called.
    let mut r = Component::Row {
        id: None,
        children: vec![],
        gap: Some("{{$target}}".into()),
        layout: None,
        breakpoints: None,
        align: None,
        justify: None,
        wrap: None,
        style: None,
    };
    r.visit_bindings(&mut |_| panic!("Row visit_bindings must not touch gap"));
}

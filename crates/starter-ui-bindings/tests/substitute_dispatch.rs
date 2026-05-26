//! substitute_tree dispatches through Bindable on every variant — not
//! just Text/Heading. Exercises Kpi.value (via source field) and
//! Chart.title via sources[].node_id, since those were the canonical
//! G1 failure cases.

use std::cell::RefCell;
use std::collections::HashMap;

use serde_json::{json, Value as JsonValue};

use starter_ui_bindings::{substitute_tree, ChildLink, EntityGraph, EvalContext};
use starter_ui_ir::{ChartSource, Component, ComponentTree};

#[derive(Default)]
struct G {
    slots: HashMap<(String, String), JsonValue>,
    children: HashMap<String, Vec<ChildLink>>,
}
impl EntityGraph for G {
    fn read_slot(&self, e: &str, s: &str) -> Option<JsonValue> {
        self.slots.get(&(e.into(), s.into())).cloned()
    }
    fn read_children(&self, e: &str) -> Vec<ChildLink> {
        self.children.get(e).cloned().unwrap_or_default()
    }
}

#[test]
fn kpi_label_substitutes_user_claim() {
    let g = G::default();
    let stack = HashMap::new();
    let mut user = serde_json::Map::new();
    user.insert("name".into(), json!("Ada"));
    let page = serde_json::Map::new();
    let ctx = EvalContext {
        graph: &g,
        target: None,
        self_id: None,
        stack: &stack,
        user: &user,
        page: &page,
        access_log: None,
        item: None,
        index: None,
        catalogue: &starter_ui_bindings::NullBag,
        locale: "en",
    };

    let mut tree = ComponentTree::new(Component::Page {
        id: "p".into(),
        title: None,
        header_actions: vec![],
        children: vec![Component::Kpi {
            id: Some("k".into()),
            label: "Chart for {{$user.name}}".into(),
            source: ChartSource::Series {
                node_id: "node-1".into(),
                slot: "value".into(),
                field: None,
            },
            value: None,
            format: None,
            intent: None,
            delta: None,
            unit_symbol: None,
            style: None,
        }],
        style: None,
        default_row_gap: None,
        default_column_gap: None,
        default_page_padding: None,
        default_max_width: None,
    });
    substitute_tree(&mut tree, &ctx).expect("substitute");
    match &tree.root {
        Component::Page { children, .. } => match &children[0] {
            Component::Kpi { label, .. } => assert_eq!(label, "Chart for Ada"),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}

#[test]
fn chart_source_node_id_substitutes_from_target() {
    let g = G::default();
    let stack = HashMap::new();
    let mut user = serde_json::Map::new();
    user.insert("device".into(), json!("dev-42"));
    let page = serde_json::Map::new();
    let log = RefCell::new(Vec::new());
    let ctx = EvalContext {
        graph: &g,
        target: None,
        self_id: None,
        stack: &stack,
        user: &user,
        page: &page,
        access_log: Some(&log),
        item: None,
        index: None,
        catalogue: &starter_ui_bindings::NullBag,
        locale: "en",
    };

    let mut tree = ComponentTree::new(Component::Page {
        id: "p".into(),
        title: None,
        header_actions: vec![],
        children: vec![Component::Chart {
            id: Some("c".into()),
            title: None,
            sources: vec![ChartSource::Series {
                node_id: "{{$user.device}}".into(),
                slot: "value".into(),
                field: None,
            }],
            kind: Default::default(),
            agg: None,
            series: vec![],
            range: None,
            page_state_key: None,
            history: None,
        }],
        style: None,
        default_row_gap: None,
        default_column_gap: None,
        default_page_padding: None,
        default_max_width: None,
    });
    substitute_tree(&mut tree, &ctx).expect("substitute");
    match &tree.root {
        Component::Page { children, .. } => match &children[0] {
            Component::Chart { sources, .. } => match &sources[0] {
                ChartSource::Series { node_id, .. } => assert_eq!(node_id, "dev-42"),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}

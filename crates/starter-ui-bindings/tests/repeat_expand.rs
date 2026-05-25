//! G3 — Repeat expansion. The expander walks the tree, evaluates
//! `Repeat.source` against the context, and replaces the Repeat with
//! one clone of `template` per array element. Synthetic `$item` and
//! `$index` are pushed into the iteration context so the substituted
//! template renders per-item values.

use std::collections::HashMap;

use serde_json::json;

use starter_ui_bindings::{expand_repeats, substitute_tree, EvalContext, NullGraph};
use starter_ui_ir::{Bindable, Component, ComponentTree};

fn ctx<'a>(
    g: &'a NullGraph,
    stack: &'a HashMap<String, String>,
    user: &'a serde_json::Map<String, serde_json::Value>,
    page: &'a serde_json::Map<String, serde_json::Value>,
) -> EvalContext<'a, NullGraph> {
    EvalContext {
        graph: g,
        target: None,
        self_id: None,
        stack,
        user,
        page,
        access_log: None,
        item: None,
        index: None,
        catalogue: &starter_ui_bindings::NullBag,
        locale: "en",
    }
}

#[test]
fn repeat_expands_one_text_per_item() {
    let g = NullGraph;
    let stack = HashMap::new();
    let mut user = serde_json::Map::new();
    user.insert("list".into(), json!(["a", "b", "c"]));
    let page = serde_json::Map::new();
    let c = ctx(&g, &stack, &user, &page);

    let mut tree = ComponentTree::new(Component::Page {
        id: "p".into(),
        title: None,
        header_actions: vec![],
        children: vec![Component::Repeat {
            id: Some("r".into()),
            source: "$user.list".into(),
            alias: None,
            template: Box::new(Component::Text {
                id: None,
                content: "i={{$index}} v={{$item}}".into(),
                intent: None,
                style: None,
            }),
        }],
        style: None,
        default_row_gap: None,
        default_column_gap: None,
        default_page_padding: None,
        default_max_width: None,
    });

    expand_repeats(&mut tree, &c).expect("expand");
    substitute_tree(&mut tree, &c).expect("substitute");

    match &tree.root {
        Component::Page { children, .. } => {
            assert_eq!(children.len(), 3);
            let texts: Vec<&str> = children
                .iter()
                .map(|c| match c {
                    Component::Text { content, .. } => content.as_str(),
                    _ => panic!("expected Text"),
                })
                .collect();
            assert_eq!(texts, vec!["i=0 v=a", "i=1 v=b", "i=2 v=c"]);
            // Each clone carries a synthetic id derived from the
            // Repeat's id ("r") and the iteration index.
            let ids: Vec<Option<&str>> = children.iter().map(|c| c.id()).collect();
            assert_eq!(ids, vec![Some("r-0"), Some("r-1"), Some("r-2")]);
        }
        _ => unreachable!(),
    }
}

#[test]
fn repeat_with_empty_source_collapses_to_zero_children() {
    let g = NullGraph;
    let stack = HashMap::new();
    let mut user = serde_json::Map::new();
    user.insert("list".into(), json!([]));
    let page = serde_json::Map::new();
    let c = ctx(&g, &stack, &user, &page);

    let mut tree = ComponentTree::new(Component::Page {
        id: "p".into(),
        title: None,
        header_actions: vec![],
        children: vec![Component::Repeat {
            id: None,
            source: "$user.list".into(),
            alias: None,
            template: Box::new(Component::Text {
                id: None,
                content: "x".into(),
                intent: None,
                style: None,
            }),
        }],
        style: None,
        default_row_gap: None,
        default_column_gap: None,
        default_page_padding: None,
        default_max_width: None,
    });
    expand_repeats(&mut tree, &c).expect("expand");
    match &tree.root {
        Component::Page { children, .. } => assert!(children.is_empty()),
        _ => unreachable!(),
    }
}


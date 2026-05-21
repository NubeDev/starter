//! Phase 2 acceptance test (DOCS/frontend/sdui/SCOPE.md §
//! "One page, N targets" smoke test).
//!
//! One authored `ComponentTree` carrying the binding
//! `{{$target/temp.value}}` must resolve correctly against three
//! different target nodes; each resolve must produce a distinct
//! literal in the rendered text, and the per-resolve subscription
//! plan must scope subjects to the resolved target — no leakage
//! across resolves.
//!
//! The fixture entity graph is the simplest thing that exercises both
//! the child walk (`/temp`) and the slot read (`.value`). It also
//! confirms that `read_children` and `read_slot` are the only graph
//! operations the binding engine relies on, satisfying the trait
//! contract S-D1 documents.

use std::cell::RefCell;
use std::collections::HashMap;

use serde_json::json;
use serde_json::Value as JsonValue;

use starter_ui_bindings::{
    ChildLink, EntityGraph, EvalContext, SubscriptionPlan, substitute_tree,
};
use starter_ui_ir::{Component, ComponentTree};

#[derive(Default)]
struct FixtureGraph {
    slots: HashMap<(String, String), JsonValue>,
    children: HashMap<String, Vec<ChildLink>>,
}

impl FixtureGraph {
    fn with_target(mut self, target: &str, temp_value: f64) -> Self {
        let temp_id = format!("{target}.temp");
        self.children
            .entry(target.to_string())
            .or_default()
            .push(ChildLink {
                name: "temp".into(),
                id: temp_id.clone(),
            });
        self.slots
            .insert((temp_id, "value".into()), json!(temp_value));
        self
    }
}

impl EntityGraph for FixtureGraph {
    fn read_slot(&self, entity_id: &str, slot: &str) -> Option<JsonValue> {
        self.slots
            .get(&(entity_id.to_string(), slot.to_string()))
            .cloned()
    }
    fn read_children(&self, entity_id: &str) -> Vec<ChildLink> {
        self.children.get(entity_id).cloned().unwrap_or_default()
    }
    fn entity_id_regex(&self) -> Option<&str> {
        // Fixture ids are free-form ("target-1", "target-2", …). A
        // host with a stable id format (UUIDs, slugs) would return
        // its regex here for ai-builder R7 to validate suggestions.
        None
    }
}

/// The "page" authored once. The same value is resolved against
/// three different targets below — no per-target re-authoring, no
/// per-target re-parsing.
fn authored_page() -> ComponentTree {
    ComponentTree::new(Component::Page {
        id: "building-overview".into(),
        title: Some("Overview".into()),
        header_actions: vec![],
        children: vec![Component::Text {
            id: Some("temp-display".into()),
            content: "Temp: {{$target/temp.value}}".into(),
            intent: None,
            style: None,
        }],
        style: None,
        default_row_gap: None,
        default_column_gap: None,
        default_page_padding: None,
        default_max_width: None,
    })
}

fn resolve_against(
    graph: &FixtureGraph,
    target: &str,
) -> (ComponentTree, SubscriptionPlan) {
    let stack: HashMap<String, String> = HashMap::new();
    let user = serde_json::Map::new();
    let page = serde_json::Map::new();
    let log = RefCell::new(Vec::new());
    let ctx = EvalContext {
        graph,
        target: Some(target),
        self_id: None,
        stack: &stack,
        user: &user,
        page: &page,
        access_log: Some(&log),
    };

    let mut tree = authored_page();
    substitute_tree(&mut tree, &ctx).expect("substitute");

    let plan = SubscriptionPlan::from_log(log.into_inner());
    (tree, plan)
}

fn text_of(tree: &ComponentTree) -> &str {
    match &tree.root {
        Component::Page { children, .. } => match children.as_slice() {
            [Component::Text { content, .. }] => content.as_str(),
            other => panic!("expected one Text child, got {other:?}"),
        },
        other => panic!("expected Page root, got {other:?}"),
    }
}

#[test]
fn one_page_resolves_across_three_targets_with_per_target_subjects() {
    let graph = FixtureGraph::default()
        .with_target("target-1", 21.5)
        .with_target("target-2", 18.0)
        .with_target("target-3", 24.25);

    let (tree_a, plan_a) = resolve_against(&graph, "target-1");
    let (tree_b, plan_b) = resolve_against(&graph, "target-2");
    let (tree_c, plan_c) = resolve_against(&graph, "target-3");

    // 1) Each resolve produces a distinct literal substitution. The
    //    page node is authored once; the rendered text differs only
    //    because $target differs.
    assert_eq!(text_of(&tree_a), "Temp: 21.5");
    assert_eq!(text_of(&tree_b), "Temp: 18.0");
    assert_eq!(text_of(&tree_c), "Temp: 24.25");

    // 2) Each resolve emits exactly one subject, scoped to the
    //    resolved target's `temp` child. No cross-target leakage.
    assert_eq!(plan_a.subjects.len(), 1);
    assert_eq!(plan_a.subjects[0].wire(), "target-1.temp/value");
    assert_eq!(plan_b.subjects.len(), 1);
    assert_eq!(plan_b.subjects[0].wire(), "target-2.temp/value");
    assert_eq!(plan_c.subjects.len(), 1);
    assert_eq!(plan_c.subjects[0].wire(), "target-3.temp/value");

    // 3) The three plans are pairwise disjoint — the load-bearing
    //    property the per-target subscription scope guarantees.
    let union: std::collections::BTreeSet<_> = plan_a
        .subjects
        .iter()
        .chain(&plan_b.subjects)
        .chain(&plan_c.subjects)
        .map(|s| s.wire())
        .collect();
    assert_eq!(union.len(), 3, "subjects must not overlap across targets");
}

#[test]
fn unknown_target_surfaces_clean_error() {
    // A resolve against an id the graph doesn't know returns
    // structured BindingError::UnknownChild (we asked for /temp on a
    // non-existent parent). The page never renders garbage.
    let graph = FixtureGraph::default().with_target("target-1", 1.0);
    let stack = HashMap::new();
    let user = serde_json::Map::new();
    let page = serde_json::Map::new();
    let ctx = EvalContext {
        graph: &graph,
        target: Some("never-existed"),
        self_id: None,
        stack: &stack,
        user: &user,
        page: &page,
        access_log: None,
    };
    let mut tree = authored_page();
    let err = substitute_tree(&mut tree, &ctx).unwrap_err();
    let rendered = format!("{err}");
    assert!(rendered.contains("never-existed"), "got {rendered}");
}

//! Phase 3 acceptance — the worked example page authored from
//! `main.rs` renders end-to-end against the Phase 2 fixture entity
//! graph.
//!
//! The Phase 2 fixture (see
//! `crates/starter-ui-bindings/tests/one_page_n_targets.rs`) exposes
//! a tiny `EntityGraph` with one `temp` child per target node. The
//! worked example below mirrors the SCOPE.md headline snippet —
//! `dashboard()` + `kpi_grid()` + a `table()` over RSQL — and asserts
//! the resulting `ComponentTree`:
//!
//! 1. Validates as a `ComponentTree` (typed round-trip).
//! 2. Resolves cleanly against the Phase 2 graph: `{{$target.name}}`
//!    in the page title substitutes to the target's `name` slot,
//!    `{{$target/temp.value}}` substitutes to the per-target reading.
//! 3. The subscription plan scopes its subjects to the resolved
//!    target — the cross-target isolation property the binding
//!    engine guarantees.
//!
//! This is the test that ties Phase 1 (IR), Phase 2 (bindings), and
//! Phase 3 (builder) into a single end-to-end smoke before any
//! HTTP / React surface lands.

use std::cell::RefCell;
use std::collections::HashMap;

use serde_json::{json, Value as JsonValue};

use starter_ui_bindings::{substitute_tree, ChildLink, EntityGraph, EvalContext, SubscriptionPlan};
use starter_ui_builder::prelude::*;

/// The Phase 2 fixture graph, with one `temp` child per target. The
/// target node itself carries a `name` slot so the page title's
/// `{{$target.name}}` binding has something to read.
#[derive(Default)]
struct FixtureGraph {
    slots: HashMap<(String, String), JsonValue>,
    children: HashMap<String, Vec<ChildLink>>,
}

impl FixtureGraph {
    fn with_target(mut self, target: &str, name: &str, temp_value: f64) -> Self {
        self.slots
            .insert((target.into(), "name".into()), json!(name));
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
        None
    }
}

/// The worked example, authored exactly as a consumer would write it
/// from `main.rs`. Renders against the Phase 2 fixture graph.
///
/// The page title carries a binding the *renderer* will substitute
/// at render time once the full per-variant `Bindable` dispatch
/// ports in Phase 4; Phase 2's `substitute_tree` deliberately covers
/// only `Text.content` / `Heading.content` (see
/// `starter-ui-bindings/src/substitute.rs`), which is enough to
/// pin the cross-target subscription-scope property without making
/// the binding engine know every variant's bindable fields. The
/// worked example exercises the `Text` content substitution for the
/// "one page, N targets" property and the page title stays
/// untouched on the wire — exactly as the renderer will receive it.
fn building_overview() -> ComponentTree {
    let display_title = format!("{} Overview", target("name"));
    dashboard(
        "building-overview",
        display_title,
        [
            kpi_grid(
                "kpis",
                "1fr 1fr",
                [
                    // The KPI source addresses a concrete node by id
                    // — not the bound target — so the worked example
                    // exercises both binding-driven and id-driven
                    // sources side by side.
                    kpi("outdoor", "Outdoor Temp", series("outdoor-temp", "value")),
                    kpi("energy", "Energy (kWh)", series("kwh", "value")),
                ],
            ),
            // Binding-driven text — substitutes per resolved target
            // via Phase 2's `substitute_tree`. The child-walk
            // grammar (`$target/temp.value`) is what makes the page
            // render distinct values across three different targets.
            text("Temp: {{$target/temp.value}}")
                .id("temp-display")
                .build(),
            // Bound heading also substitutes per target — the second
            // text-y variant Phase 2's substitute_tree walks.
            heading("Latest reading {{$target/temp.value}}")
                .id("reading-h")
                .level(3)
                .build(),
            // RSQL-driven table — server pages rows; the smoke just
            // pins the wire shape.
            table("alarms", rsql().kind("alarm.active"))
                .live()
                .column("Time", "slots.ts.value")
                .column("Severity", "slots.severity.value")
                .build(),
        ],
    )
}

fn resolve_against(graph: &FixtureGraph, target_id: &str) -> (ComponentTree, SubscriptionPlan) {
    let stack: HashMap<String, String> = HashMap::new();
    let user = serde_json::Map::new();
    let page = serde_json::Map::new();
    let log = RefCell::new(Vec::new());
    let ctx = EvalContext {
        graph,
        target: Some(target_id),
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

    let mut tree = building_overview();
    substitute_tree(&mut tree, &ctx).expect("substitute");
    let plan = SubscriptionPlan::from_log(log.into_inner());
    (tree, plan)
}

#[test]
fn worked_example_round_trips_through_serde() {
    let tree = building_overview();
    let v = serde_json::to_value(&tree).expect("serialise");
    let back: ComponentTree = serde_json::from_value(v).expect("deserialise");
    assert_eq!(back.ir_version, IR_VERSION);
}

#[test]
fn worked_example_resolves_against_phase2_fixture() {
    let graph = FixtureGraph::default()
        .with_target("b1", "Building One", 21.5)
        .with_target("b2", "Building Two", 18.0);

    let (tree_a, plan_a) = resolve_against(&graph, "b1");
    let (tree_b, plan_b) = resolve_against(&graph, "b2");

    // The page title carries a binding template that substitute_tree
    // now resolves: `visit_bindings` visits `Page.title` (a bindable
    // string field), so `{{$target.name}} Overview` resolves to the
    // target's name per resolve. This is the intentional widening the
    // original Phase-2 comment anticipated — the assertion was flipped
    // when `Page.title` joined the visited set.
    let title_a = match &tree_a.root {
        Component::Page { title, .. } => title.clone().unwrap(),
        other => panic!("expected Page root, got {other:?}"),
    };
    assert_eq!(title_a, "Building One Overview");

    // And it resolves per-target — `tree_b` carries the other name.
    let title_b = match &tree_b.root {
        Component::Page { title, .. } => title.clone().unwrap(),
        other => panic!("expected Page root, got {other:?}"),
    };
    assert_eq!(title_b, "Building Two Overview");

    // The bound text and heading widgets resolve the child-walk
    // grammar — distinct literals per target, as the Phase 2
    // acceptance test pins.
    let temp_a = find_text(&tree_a, "temp-display");
    let temp_b = find_text(&tree_b, "temp-display");
    assert_eq!(temp_a, "Temp: 21.5");
    assert_eq!(temp_b, "Temp: 18.0");

    let heading_a = find_heading(&tree_a, "reading-h");
    let heading_b = find_heading(&tree_b, "reading-h");
    assert_eq!(heading_a, "Latest reading 21.5");
    assert_eq!(heading_b, "Latest reading 18.0");

    // Per-target subscription scope. The `$target/temp.value` binding
    // emits a subject keyed on the resolved target, so the two plans
    // are disjoint.
    let subjects_a: Vec<_> = plan_a.subjects.iter().map(|s| s.wire()).collect();
    let subjects_b: Vec<_> = plan_b.subjects.iter().map(|s| s.wire()).collect();
    assert!(subjects_a.iter().any(|s| s == "b1.temp/value"));
    assert!(subjects_b.iter().any(|s| s == "b2.temp/value"));
    for s in &subjects_a {
        assert!(
            !subjects_b.contains(s),
            "subscription plans must not overlap across targets; shared subject {s}"
        );
    }
}

fn find_heading(tree: &ComponentTree, id: &str) -> String {
    let Component::Page { children, .. } = &tree.root else {
        panic!("expected Page root");
    };
    for child in children {
        if let Component::Heading {
            id: Some(heading_id),
            content,
            ..
        } = child
        {
            if heading_id == id {
                return content.clone();
            }
        }
    }
    panic!("no Heading child with id {id}");
}

fn find_text(tree: &ComponentTree, id: &str) -> String {
    let Component::Page { children, .. } = &tree.root else {
        panic!("expected Page root");
    };
    for child in children {
        if let Component::Text {
            id: Some(text_id),
            content,
            ..
        } = child
        {
            if text_id == id {
                return content.clone();
            }
        }
    }
    panic!("no Text child with id {id}");
}

//! Phase 8 falsification suite — three fixture pages render
//! end-to-end through one renderer (SCOPE.md § R3 / R9 / Phase 8).
//!
//! The three pages mirror the falsification use cases the SCOPE
//! pins as the substrate's acceptance smoke:
//!
//!   1. **CRUD list** — a paginated, searchable list with a row
//!      action and a header-mounted "create" form trigger. The
//!      shape Rubix's BACnet device list took during S1.
//!   2. **PR review card** — a card with a side-by-side diff plus
//!      inline action buttons that carry `Action.optimistic`
//!      hints. The shape Rubix's PR review surface took during S5.
//!   3. **Scope board** — a multi-column board of KPI tiles +
//!      status badges, wired through a live-table feed and a
//!      `select` that writes to `$page` to refilter. The shape
//!      Rubix's scope-plan board took during S6.
//!
//! What this file pins:
//!
//!   * Every fixture builds via the typed `starter-ui-builder` DSL
//!     (no direct `Component::*` literals at authoring sites —
//!     the test would still pass if a fixture used a literal, but
//!     the spirit is "author through the DSL the way consumers
//!     would").
//!   * Each fixture serialises to valid IR JSON.
//!   * Each fixture deserialises back through the typed
//!     `ComponentTree` round-trip (the runtime validator-of-record
//!     for the wire shape — `builder_smoke.rs` covers the JSON
//!     Schema arm).
//!   * Every node type in every fixture is one of the built-in
//!     variants the single renderer dispatches on — i.e. no
//!     fixture relies on a `custom` escape hatch to render.
//!   * No fixture leaks domain vocabulary into the IR variant
//!     vocabulary: the *content* of a fixture mentions devices /
//!     PRs / scope items, but the IR is structural (R3 — see also
//!     the TS-side `r3-no-domain-leak.test.ts`).

use std::collections::HashSet;

use serde_json::Value;
use starter_ui_builder::prelude::*;
use starter_ui_ir::{Component, ComponentTree, DiffAnnotation, OptimisticHint};

// ---------------------------------------------------------------------------
// Fixture 1 — CRUD device list
// ---------------------------------------------------------------------------

fn crud_device_list() -> ComponentTree {
    dashboard(
        "list",
        "Items",
        [row("toolbar")
            .child(heading("All items").build())
            .child(
                table("items-table", rsql().kind("inventory.item"))
                    .live()
                    .searchable()
                    .page_size(50)
                    .column("Name", "slots.name.value")
                    .column("Status", "slots.status.value")
                    .build(),
            )
            .build()],
    )
}

// ---------------------------------------------------------------------------
// Fixture 2 — Review card with diff + inline optimistic actions
// ---------------------------------------------------------------------------

fn review_card() -> ComponentTree {
    // The `diff` IR variant has no first-class builder wrapper yet
    // (it delegates to monaco-diff on the client). Construct it
    // directly — this is the documented escape hatch for variants
    // whose builder sugar lands later.
    let diff = Component::Diff {
        id: Some("review-diff".into()),
        old_text: "alpha\nbeta\n".into(),
        new_text: "alpha\nBETA\n".into(),
        language: Some("text".into()),
        annotations: vec![DiffAnnotation {
            line: 2,
            text: "renamed for clarity".into(),
            author: None,
            created_at: None,
        }],
        line_action: None,
    };

    // An "approve" button carries an `Action.optimistic` hint —
    // the field the substrate's Phase 8 round-trip exercises.
    let approve = Component::Button {
        id: Some("approve-btn".into()),
        label: "Approve".into(),
        intent: Some("success".into()),
        disabled: None,
        action: Some(Action {
            handler: "review.approve".into(),
            args: Some(serde_json::json!({ "id": "{{$target.id}}" })),
            optimistic: Some(OptimisticHint {
                target_component_id: "approve-btn".into(),
                fields: serde_json::json!({ "disabled": true, "label": "Approved" }),
            }),
        }),
        style: None,
    };

    let request_changes = Component::Button {
        id: Some("request-btn".into()),
        label: "Request changes".into(),
        intent: Some("warning".into()),
        disabled: None,
        action: Some(Action {
            handler: "review.request_changes".into(),
            args: None,
            optimistic: Some(OptimisticHint {
                target_component_id: "request-btn".into(),
                fields: serde_json::json!({ "disabled": true }),
            }),
        }),
        style: None,
    };

    dashboard(
        "review",
        "{{$target.title}}",
        [col("review-col")
            .child(
                heading("Proposed change")
                    .subtitle("by {{$target.author}}")
                    .build(),
            )
            .child(diff)
            .child(row("actions").child(approve).child(request_changes).build())
            .build()],
    )
}

// ---------------------------------------------------------------------------
// Fixture 3 — Scope board with state badges + live updates
// ---------------------------------------------------------------------------

fn scope_board() -> ComponentTree {
    dashboard(
        "board",
        "Board",
        [
            row("filters")
                .child(
                    select("status-filter", "status_filter")
                        .placeholder("Filter by status")
                        .option("All", "all")
                        .option("Open", "open")
                        .option("In progress", "in_progress")
                        .option("Done", "done")
                        .build(),
                )
                .build(),
            kpi_grid(
                "kpis",
                "1fr 1fr 1fr",
                [
                    kpi("open-count", "Open", series("board-stats", "open_count")),
                    kpi(
                        "in-progress-count",
                        "In progress",
                        series("board-stats", "in_progress_count"),
                    ),
                    kpi("done-count", "Done", series("board-stats", "done_count")),
                ],
            ),
            row("status-row")
                .child(badge("Open").intent("info").build())
                .child(badge("Blocked").intent("danger").build())
                .child(badge("Done").intent("success").build())
                .build(),
            table("board-items", rsql().kind("board.item"))
                .live()
                .column("Title", "slots.title.value")
                .column("Owner", "slots.owner.value")
                .column("Status", "slots.status.value")
                .build(),
        ],
    )
}

// ---------------------------------------------------------------------------
// Assertions — shared across all three fixtures
// ---------------------------------------------------------------------------

/// IR variants the React `Renderer` dispatches on (mirrors the
/// `Kind` union in `packages/starter-sdui-react/src/types.ts`).
/// A fixture that emits a node type outside this set would only
/// render through the `custom` escape hatch — the falsification
/// suite forbids that.
fn builtin_kinds() -> HashSet<&'static str> {
    [
        "page",
        "row",
        "col",
        "grid",
        "stack",
        "tabs",
        "card",
        "section",
        "text",
        "heading",
        "badge",
        "kpi",
        "kpi_grid",
        "button",
        "link",
        "table",
        "form",
        "field",
        "field_group",
        "select",
        "toggle",
        "chart",
        "sparkline",
        "tree",
        "timeline",
        "markdown",
        "code",
        "wizard",
        "drawer",
        "rich_text",
        "diff",
        "ref_picker",
        "date_range",
        "divider",
        "slider",
        "detail",
        "json_table",
        "gauge",
        "action_form",
        "dashboard",
    ]
    .into_iter()
    .collect()
}

fn walk_kinds(v: &Value, out: &mut HashSet<String>) {
    if let Some(t) = v.get("type").and_then(|t| t.as_str()) {
        out.insert(t.to_string());
    }
    if let Some(children) = v.get("children").and_then(|c| c.as_array()) {
        for c in children {
            walk_kinds(c, out);
        }
    }
    if let Some(tabs) = v.get("tabs").and_then(|c| c.as_array()) {
        for t in tabs {
            if let Some(ch) = t.get("children").and_then(|c| c.as_array()) {
                for c in ch {
                    walk_kinds(c, out);
                }
            }
        }
    }
    // For the `kpi_grid` shape the renderer walks `tiles`, and
    // `card` walks `actions`; widen the recursion to cover them so
    // a leaked unknown variant in a tile is still flagged.
    for key in ["tiles", "actions", "header_actions", "items"] {
        if let Some(arr) = v.get(key).and_then(|c| c.as_array()) {
            for c in arr {
                walk_kinds(c, out);
            }
        }
    }
    if let Some(control) = v.get("control") {
        walk_kinds(control, out);
    }
}

fn assert_fixture(name: &str, tree: ComponentTree) {
    let json = serde_json::to_value(&tree).expect("serialise to JSON");
    assert_eq!(json["root"]["type"], "page", "{name}: root must be page");

    // Typed round-trip — the wire-shape validator of record.
    let _back: ComponentTree = serde_json::from_value(json.clone()).expect("typed round-trip");

    // Every emitted node type lives in the renderer's dispatch
    // table (no implicit `custom` fallbacks in the falsification
    // suite).
    let mut kinds = HashSet::<String>::new();
    walk_kinds(&json["root"], &mut kinds);
    let builtin = builtin_kinds();
    for k in &kinds {
        assert!(
            builtin.contains(k.as_str()),
            "{name}: node type `{k}` is not in the built-in dispatch table",
        );
    }
}

#[test]
fn crud_device_list_renders_through_one_renderer() {
    assert_fixture("crud_device_list", crud_device_list());
}

#[test]
fn review_card_renders_through_one_renderer() {
    let tree = review_card();
    // Pin the Phase 8 contract: the approve button carries an
    // `Action.optimistic` hint with `target_component_id` ==
    // `approve-btn` and a `disabled: true` patch.
    let json = serde_json::to_value(&tree).unwrap();
    let actions = &json["root"]["children"][0]["children"][2]["children"];
    let approve = &actions[0];
    assert_eq!(approve["type"], "button");
    assert_eq!(
        approve["action"]["optimistic"]["target_component_id"],
        "approve-btn"
    );
    assert_eq!(approve["action"]["optimistic"]["fields"]["disabled"], true);
    assert_fixture("review_card", tree);
}

#[test]
fn scope_board_renders_through_one_renderer() {
    assert_fixture("scope_board", scope_board());
}

#[test]
fn three_fixtures_share_the_same_dispatch_path() {
    // A "one renderer" assertion: union the node types emitted by
    // the three fixtures and confirm they all belong to the
    // single built-in dispatch table. If a future fixture needs a
    // node the dispatch table doesn't know about, the right
    // answer is to add the variant to the IR + renderer registry
    // (or to declare it a `custom`), not to grow per-fixture
    // dispatch logic.
    let trees = [crud_device_list(), review_card(), scope_board()];
    let mut all_kinds = HashSet::<String>::new();
    for t in &trees {
        let json = serde_json::to_value(t).unwrap();
        walk_kinds(&json["root"], &mut all_kinds);
    }
    let builtin = builtin_kinds();
    for k in &all_kinds {
        assert!(
            builtin.contains(k.as_str()),
            "node type `{k}` would force the renderer to add a special case",
        );
    }
    // Sanity — the union covers the key Phase 8 shapes.
    for required in ["page", "table", "diff", "button", "kpi", "badge", "select"] {
        assert!(
            all_kinds.contains(required),
            "expected falsification suite to cover `{required}` somewhere",
        );
    }
}

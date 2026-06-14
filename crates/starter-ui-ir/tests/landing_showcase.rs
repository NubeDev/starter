//! Validates the bundled `landing-showcase.json` page-builder demo
//! deserialises into a `ComponentTree` and exercises the V2.0 content
//! blocks (hero / image / spacer / rich_text / card / section / CTA
//! button). If a field name on the wire drifts from the IR, this test
//! fails — keeping the demo a real, loadable artifact rather than
//! documentation that silently rots.

use starter_ui_ir::{Component, ComponentTree};

const SHOWCASE: &str = include_str!(
    "../../../rubix/crates/rubix-flows/dashboards/landing-showcase.json"
);

fn walk<'a>(node: &'a Component, out: &mut Vec<&'a str>) {
    out.push(match node {
        Component::Page { .. } => "page",
        Component::Hero { .. } => "hero",
        Component::Section { .. } => "section",
        Component::Card { .. } => "card",
        Component::Image { .. } => "image",
        Component::Spacer { .. } => "spacer",
        Component::RichText { .. } => "rich_text",
        Component::Button { .. } => "button",
        Component::Row { .. } => "row",
        Component::Col { .. } => "col",
        Component::Kpi { .. } => "kpi",
        Component::Chart { .. } => "chart",
        _ => "other",
    });
    // Descend into the container variants the demo uses.
    let children: &[Component] = match node {
        Component::Page { children, .. }
        | Component::Hero { children, .. }
        | Component::Section { children, .. }
        | Component::Card { children, .. }
        | Component::Row { children, .. }
        | Component::Col { children, .. } => children,
        _ => &[],
    };
    for c in children {
        walk(c, out);
    }
}

#[test]
fn landing_showcase_deserialises_and_uses_new_blocks() {
    let tree: ComponentTree =
        serde_json::from_str(SHOWCASE).expect("landing-showcase.json must be valid IR");
    assert_eq!(tree.ir_version, 5);

    let mut kinds = Vec::new();
    walk(&tree.root, &mut kinds);

    // The demo must actually exercise the new page-builder vocabulary,
    // not silently degrade to plain rows/cols.
    for required in ["hero", "section", "card", "image", "spacer", "rich_text", "button"] {
        assert!(
            kinds.contains(&required),
            "demo page is missing a `{required}` block (kinds present: {kinds:?})"
        );
    }
    // And no node should have fallen through to the catch-all, which
    // would mean an unrecognised type slipped onto the wire.
    assert!(
        !kinds.contains(&"other"),
        "demo page contains an unexpected component kind: {kinds:?}"
    );
}

#[test]
fn cta_button_carries_href_and_variant() {
    let tree: ComponentTree = serde_json::from_str(SHOWCASE).unwrap();
    let mut found_cta = false;
    let mut stack = vec![&tree.root];
    while let Some(node) = stack.pop() {
        if let Component::Button { href, variant, .. } = node {
            if href.is_some() {
                found_cta = true;
                assert!(variant.is_some(), "CTA button should declare a variant");
            }
        }
        let children: &[Component] = match node {
            Component::Page { children, .. }
            | Component::Hero { children, .. }
            | Component::Section { children, .. }
            | Component::Card { children, .. }
            | Component::Row { children, .. }
            | Component::Col { children, .. } => children,
            _ => &[],
        };
        for c in children {
            stack.push(c);
        }
    }
    assert!(found_cta, "demo page must contain at least one href CTA button");
}

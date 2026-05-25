//! G2 — Qualifier grammar: trailing `?` makes a binding optional
//! (lookup errors collapse to empty), trailing `!` is the explicit
//! required form (identical to default for now).

use std::collections::HashMap;

use starter_ui_bindings::{substitute_text, Binding, EvalContext, NullGraph, Qualifier};

#[test]
fn parses_optional_qualifier() {
    let b = Binding::parse("$user.x?").unwrap();
    assert_eq!(b.qualifier, Qualifier::Optional);
    // The `?` is stripped — the ident is just `x`.
    assert_eq!(b.steps.len(), 1);
}

#[test]
fn parses_required_qualifier() {
    let b = Binding::parse("$user.x!").unwrap();
    assert_eq!(b.qualifier, Qualifier::Required);
}

#[test]
fn parses_default_qualifier() {
    let b = Binding::parse("$user.x").unwrap();
    assert_eq!(b.qualifier, Qualifier::Default);
}

#[test]
fn optional_missing_claim_collapses_to_empty() {
    let g = NullGraph;
    let stack = HashMap::new();
    let user = serde_json::Map::new(); // empty — `missing` is not present
    let page = serde_json::Map::new();
    let ctx = EvalContext {
        graph: &g,
        target: None,
        self_id: None,
        stack: &stack,
        user: &user,
        page: &page,
        access_log: None,
    };
    let out = substitute_text("hi {{$user.missing?}}!", &ctx).unwrap();
    assert_eq!(out, "hi !");
}

#[test]
fn non_optional_missing_target_errors() {
    let g = NullGraph;
    let stack = HashMap::new();
    let user = serde_json::Map::new();
    let page = serde_json::Map::new();
    let ctx = EvalContext {
        graph: &g,
        target: None,
        self_id: None,
        stack: &stack,
        user: &user,
        page: &page,
        access_log: None,
    };
    // $target with no target in scope is an error; without `?` it
    // surfaces structurally.
    assert!(substitute_text("{{$target/x.y}}", &ctx).is_err());
    // With `?` it collapses.
    assert_eq!(
        substitute_text("v={{$target/x.y?}}", &ctx).unwrap(),
        "v="
    );
}

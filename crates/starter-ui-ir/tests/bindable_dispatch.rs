//! Exercises the hand-written `Bindable for Component` dispatch that
//! replaces Rubix's `#[derive(Bindable)]` proc macro. The per-variant
//! coerce policy is unit-tested inside `component.rs`; this test
//! covers the three plumbing methods (`id`, `read_binding`,
//! `write_bindings`) across required-id, optional-id, and
//! multi-write variants.

use starter_ui_ir::{Bindable, BindingSpec, Bindings, Component};

#[test]
fn required_id_string_variant_returns_some() {
    let c = Component::Toggle {
        id: "switch-1".into(),
        bind: Bindings::one(BindingSpec::Short("$target.enabled".into())),
        label: Some("On".into()),
        value: None,
        style: None,
    };
    assert_eq!(c.id(), Some("switch-1"));
    assert_eq!(c.read_binding().map(BindingSpec::slot_expr), Some("$target.enabled"));
    assert_eq!(c.write_bindings().len(), 1);
}

#[test]
fn optional_id_unset_returns_none() {
    let c = Component::Row {
        id: None,
        children: vec![],
        gap: None,
        layout: None,
        breakpoints: None,
        align: None,
        justify: None,
        wrap: None,
        style: None,
    };
    assert_eq!(c.id(), None);
    assert!(c.read_binding().is_none());
    assert!(c.write_bindings().is_empty());
}

#[test]
fn multi_write_fanout_preserves_declaration_order() {
    // Toggle with a fan-out: SDUI-VALUES.md §3.1 says writes are
    // plural and ordered; the first entry is also the read source.
    let c = Component::Toggle {
        id: "fan-1".into(),
        bind: Bindings(vec![
            BindingSpec::Short("$target.enabled".into()),
            BindingSpec::Short("$mirror.enabled".into()),
        ]),
        label: Some("Mirror".into()),
        value: None,
        style: None,
    };
    let writes = c.write_bindings();
    assert_eq!(writes.len(), 2);
    assert_eq!(writes[0].slot_expr(), "$target.enabled");
    assert_eq!(writes[1].slot_expr(), "$mirror.enabled");
    // Read is the first entry by convention.
    assert_eq!(c.read_binding().map(BindingSpec::slot_expr), Some("$target.enabled"));
}

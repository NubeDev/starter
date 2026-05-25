//! G5/C5 — `Component::synthetic_id` + `assign_synthetic_id`.

use starter_ui_ir::{Bindable, Component};

#[test]
fn synthetic_id_is_stable_and_index_based() {
    assert_eq!(Component::synthetic_id("cards", 0), "cards-0");
    assert_eq!(Component::synthetic_id("cards", 42), "cards-42");
}

#[test]
fn assign_synthetic_id_fills_only_when_blank() {
    let mut c = Component::Text {
        id: None,
        content: "hi".into(),
        intent: None,
        style: None,
    };
    c.assign_synthetic_id("parent", 3);
    assert_eq!(c.id(), Some("parent-3"));

    let mut c = Component::Text {
        id: Some("authored".into()),
        content: "hi".into(),
        intent: None,
        style: None,
    };
    c.assign_synthetic_id("parent", 3);
    assert_eq!(c.id(), Some("authored"));
}

#[test]
fn assign_synthetic_id_fills_required_id_when_empty() {
    let mut c = Component::Toggle {
        id: String::new(),
        bind: starter_ui_ir::Bindings::one(starter_ui_ir::BindingSpec::Short(
            "$target.enabled".into(),
        )),
        label: None,
        value: None,
        style: None,
    };
    c.assign_synthetic_id("group", 1);
    assert_eq!(c.id(), Some("group-1"));
}

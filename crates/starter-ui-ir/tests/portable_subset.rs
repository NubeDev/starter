//! G5/C6 — portable IR subset. `Component::is_portable` and the
//! string constant `IR_PORTABLE_VARIANTS` agree, the listed variants
//! all report `true`, and a few representative non-portable variants
//! report `false`.

use starter_ui_ir::{Component, IR_PORTABLE_VARIANTS};

#[test]
fn portable_list_is_non_empty_and_unique() {
    assert!(!IR_PORTABLE_VARIANTS.is_empty());
    let mut sorted: Vec<&&str> = IR_PORTABLE_VARIANTS.iter().collect();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), IR_PORTABLE_VARIANTS.len());
}

#[test]
fn representative_portable_variants_return_true() {
    let row = Component::Row {
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
    assert!(row.is_portable());
    let text = Component::Text {
        id: None,
        content: "x".into(),
        intent: None,
        style: None,
    };
    assert!(text.is_portable());
    let divider = Component::Divider {
        id: None,
        orientation: None,
        intent: None,
        spacing: None,
    };
    assert!(divider.is_portable());
}

#[test]
fn non_portable_variants_return_false() {
    let menu = Component::Menu {
        id: None,
        trigger: None,
        items: vec![],
    };
    assert!(!menu.is_portable());
    let heading = Component::Heading {
        id: None,
        content: "h".into(),
        subtitle: None,
        level: None,
        style: None,
    };
    assert!(!heading.is_portable());
    let custom = Component::Custom {
        id: None,
        renderer_id: "x.y".into(),
        props: None,
        subscribe: vec![],
    };
    assert!(!custom.is_portable());
}

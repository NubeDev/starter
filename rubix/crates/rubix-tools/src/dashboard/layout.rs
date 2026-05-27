//! Structural layout validation for SDUI dashboard `body_json`.
//!
//! Catches the silent layout-wrong / renders-as-column-of-cards class
//! of bugs at write time (issue #9 of
//! `rubix/docs/design/sdui/dashboard-api-usage.md`). The renderer is
//! strict but silent: a `row` whose child is another `row`, or a tree
//! whose root is `row` instead of `page`, parses fine but paints as a
//! vertical stack with no diagnostic. We reject those shapes here so
//! the tool call fails with a clear `Error::Invalid` instead.

use serde_json::Value;
use starter_spi::error::{Error, Result};

/// Validate the layout invariants the renderer relies on but does not
/// enforce:
///
/// 1. Root component type must be `page`.
/// 2. Direct children of a `row` must be `col`.
///
/// Other types (`col`, leaves) recurse without extra checks — they're
/// either layout containers whose grammar is unconstrained (a `col`
/// can hold any widget) or non-layout leaves the renderer dispatches
/// on type.
pub fn validate_layout(body: &Value) -> Result<()> {
    let root = body.get("root").ok_or_else(|| Error::Invalid {
        message: "body_json missing `root`".to_owned(),
    })?;
    let root_type = root.get("type").and_then(Value::as_str);
    if root_type != Some("page") {
        return Err(Error::Invalid {
            message: format!(
                "body_json.root.type must be `page`, got `{}`. \
                 The renderer's page chrome (title, padding, gap CSS \
                 variables) is only emitted for a `page` root; other \
                 roots render as a bare vertical stack.",
                root_type.unwrap_or("<missing>")
            ),
        });
    }
    walk(root)
}

fn walk(node: &Value) -> Result<()> {
    let ty = node.get("type").and_then(Value::as_str).unwrap_or("");
    if let Some(children) = node.get("children").and_then(Value::as_array) {
        if ty == "row" {
            for (i, child) in children.iter().enumerate() {
                let ct = child
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("<missing>");
                if ct != "col" {
                    return Err(Error::Invalid {
                        message: format!(
                            "row child #{i} has type `{ct}`, but every direct \
                             child of a `row` must be `col`. The renderer's \
                             12-column grid math assumes col children; other \
                             types render but break the layout silently."
                        ),
                    });
                }
            }
        }
        for child in children {
            walk(child)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn page(children: Value) -> Value {
        json!({ "ir_version": 5, "root": { "type": "page", "id": "p", "children": children } })
    }

    #[test]
    fn valid_page_passes() {
        let body = page(json!([
            { "type": "row", "id": "r1", "children": [
                { "type": "col", "span": 12, "children": [
                    { "type": "kpi", "id": "k1" }
                ]}
            ]}
        ]));
        assert!(validate_layout(&body).is_ok());
    }

    #[test]
    fn missing_root_rejected() {
        let err = validate_layout(&json!({"ir_version": 5})).unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }

    #[test]
    fn non_page_root_rejected() {
        let body = json!({ "ir_version": 5, "root": { "type": "row", "children": [] } });
        let err = validate_layout(&body).unwrap_err();
        match err {
            Error::Invalid { message } => assert!(message.contains("must be `page`")),
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn row_with_non_col_child_rejected() {
        let body = page(json!([
            { "type": "row", "children": [
                { "type": "row", "children": [] }
            ]}
        ]));
        let err = validate_layout(&body).unwrap_err();
        match err {
            Error::Invalid { message } => assert!(message.contains("must be `col`")),
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn nested_rows_under_cols_pass() {
        // page -> row -> col -> row -> col is legal (a row inside a col
        // is fine; only row-direct-row is the broken shape).
        let body = page(json!([
            { "type": "row", "children": [
                { "type": "col", "span": 12, "children": [
                    { "type": "row", "children": [
                        { "type": "col", "span": 6, "children": [] }
                    ]}
                ]}
            ]}
        ]));
        assert!(validate_layout(&body).is_ok());
    }
}

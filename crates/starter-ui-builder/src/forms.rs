//! Form builders — the recurring `heading + form` pair (the
//! [`action_form`] sugar) and a thin [`form()`] constructor over the
//! IR's [`Component::Form`] variant.

use serde_json::{Value as JsonValue, json};
use starter_ui_ir::{Action, Component};

// =====================================================================
// form() — direct constructor over Component::Form
// =====================================================================

/// Construct a [`Component::Form`] with a JSON-Schema form body and
/// a submit handler. The schema is double-encoded into `schema_ref`
/// as the IR contract requires (the renderer's `parseSchema()` calls
/// `JSON.parse()` to recover the object).
pub fn form(id: impl Into<String>, handler: impl Into<String>, schema: JsonValue) -> FormBuilder {
    FormBuilder {
        id: id.into(),
        schema,
        handler: handler.into(),
        bindings: None,
        submit_label: None,
    }
}

/// Builder for [`Component::Form`].
#[derive(Debug, Clone)]
pub struct FormBuilder {
    id: String,
    schema: JsonValue,
    handler: String,
    bindings: Option<JsonValue>,
    submit_label: Option<String>,
}

impl FormBuilder {
    /// Seed the form's initial data — used by edit flows that read
    /// the current record.
    pub fn bindings(mut self, bindings: JsonValue) -> Self {
        self.bindings = Some(bindings);
        self
    }

    /// Override the submit-button label.
    pub fn submit_label(mut self, label: impl Into<String>) -> Self {
        self.submit_label = Some(label.into());
        self
    }

    /// Materialise.
    pub fn build(self) -> Component {
        let schema_ref =
            serde_json::to_string(&self.schema).expect("form schema must serialise");
        Component::Form {
            id: Some(self.id),
            schema_ref,
            bindings: self.bindings,
            submit: Some(Action {
                handler: self.handler,
                args: None,
                optimistic: None,
            }),
            submit_label: self.submit_label,
        }
    }
}

// =====================================================================
// action_form — heading + form pair as raw JSON
// =====================================================================

/// Spec for one action exposed on a page.
///
/// `intent` is forwarded to the submit button (`"primary"`,
/// `"destructive"`, …). `id_prefix` defaults to the handler's last
/// `.`-separated segment when omitted.
pub struct ActionForm<'a> {
    /// Fully-qualified handler name, e.g.
    /// `"com.acme.hello.task.create"`.
    pub handler: &'a str,
    /// Heading shown above the form, e.g. `"Create Task"`.
    pub heading: &'a str,
    /// Button label.
    pub button_label: &'a str,
    /// Button intent — `"primary"`, `"destructive"`, etc.
    pub intent: &'a str,
    /// JSON Schema for the form fields. Embedded as a string in
    /// `schema_ref` (the IR contract types `schema_ref` as `String`).
    pub schema: JsonValue,
    /// Optional explicit id prefix. When `None`, derived from the
    /// last dotted segment of `handler`.
    pub id_prefix: Option<&'a str>,
}

/// Render an [`ActionForm`] as `[heading, form]` JSON values ready to
/// splice into a `col` / `row` `children` array. The form's
/// `submit_label` carries [`ActionForm::button_label`].
pub fn action_form(spec: ActionForm<'_>) -> Vec<JsonValue> {
    let prefix = spec.id_prefix.map(str::to_owned).unwrap_or_else(|| {
        spec.handler
            .rsplit('.')
            .next()
            .unwrap_or(spec.handler)
            .to_owned()
    });

    let schema_ref = serde_json::to_string(&spec.schema)
        .expect("ActionForm.schema must be a serializable JSON value");

    vec![
        json!({
            "type": "heading",
            "id": format!("h-{prefix}"),
            "content": spec.heading,
            "level": 3,
        }),
        json!({
            "type": "form",
            "id": format!("form-{prefix}"),
            "schema_ref": schema_ref,
            "submit": { "handler": spec.handler },
            "submit_label": spec.button_label,
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_builder_serialises_schema_into_schema_ref() {
        let f = form(
            "create",
            "com.acme.task.create",
            json!({ "type": "object", "properties": { "name": { "type": "string" } } }),
        )
        .submit_label("Create")
        .build();
        let v = serde_json::to_value(&f).unwrap();
        assert_eq!(v["type"], "form");
        assert_eq!(v["submit"]["handler"], "com.acme.task.create");
        assert_eq!(v["submit_label"], "Create");
        let schema_ref = v["schema_ref"].as_str().unwrap();
        let parsed: JsonValue = serde_json::from_str(schema_ref).unwrap();
        assert_eq!(parsed["type"], "object");
    }

    #[test]
    fn action_form_derives_id_prefix_from_handler_tail() {
        let out = action_form(ActionForm {
            handler: "com.acme.hello.task.create",
            heading: "Create Task",
            button_label: "Create",
            intent: "primary",
            schema: json!({ "type": "object" }),
            id_prefix: None,
        });
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["id"], "h-create");
        assert_eq!(out[1]["id"], "form-create");
        assert_eq!(out[1]["submit"]["handler"], "com.acme.hello.task.create");
    }
}

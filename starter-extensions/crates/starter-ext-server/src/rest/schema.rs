//! Minimal JSON-Schema check used by the REST adapter at request time.
//!
//! Why hand-rolled and not `jsonschema`: the crate ships no JSON-Schema
//! dependency today (the parent workspace doesn't pull one in either),
//! and the v0.1 REST adapter only needs three things to satisfy
//! "input_schema validation before the extension is invoked":
//!
//! 1. The body parses as JSON (already done by `serde_json::from_slice`).
//! 2. The top-level `type` (object / array / string / number / integer /
//!    boolean / null) matches the schema's `type`.
//! 3. If the schema is `object` and lists `required: [..]`, every
//!    required key is present.
//!
//! That covers every typo / wrong-shape mistake an extension author
//! actually makes in the wild without dragging a meta-schema validator
//! into the dependency tree. A consumer who needs full Draft-2020-12
//! semantics replaces [`SchemaCheck`] with their own implementation by
//! pre-validating before the adapter sees the body and disabling the
//! check via a future adapter knob — the manifest field is unchanged.

use serde_json::Value;

/// Pre-parsed, request-time schema check.
///
/// The `Option<…>`s reflect what an extension author actually puts in
/// `input_schema.json` for a tiny v0.1 extension. A schema with neither
/// a `type` nor `required` is accepted as a permissive pass — useful for
/// "I'm still iterating" handlers that take an arbitrary object.
#[derive(Debug, Clone, Default)]
pub struct SchemaCheck {
    /// JSON-Schema `type` (lowercased), if declared.
    pub ty: Option<String>,
    /// JSON-Schema `required` for object schemas.
    pub required: Vec<String>,
}

impl SchemaCheck {
    /// Build from a raw `serde_json::Value` schema. Returns the default
    /// (no checks) when the schema is not a JSON object — adapters keep
    /// loading so a malformed schema does not stop the registry from
    /// coming up.
    pub fn from_value(schema: &Value) -> Self {
        let obj = match schema.as_object() {
            Some(o) => o,
            None => return Self::default(),
        };
        let ty = obj
            .get("type")
            .and_then(Value::as_str)
            .map(|s| s.to_ascii_lowercase());
        let required = obj
            .get("required")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        Self { ty, required }
    }

    /// Apply the check to one request body. Returns a human-readable
    /// reason on failure suitable for the `400 Bad Request` payload the
    /// REST adapter emits.
    pub fn check(&self, body: &Value) -> Result<(), String> {
        if let Some(ty) = &self.ty {
            if !type_matches(ty, body) {
                return Err(format!(
                    "input does not match schema `type: {ty}` (got {})",
                    type_name(body),
                ));
            }
        }
        if let Some(obj) = body.as_object() {
            for key in &self.required {
                if !obj.contains_key(key) {
                    return Err(format!("input missing required field `{key}`"));
                }
            }
        } else if !self.required.is_empty() {
            return Err(format!(
                "input must be an object — schema declares required fields {:?}",
                self.required
            ));
        }
        Ok(())
    }
}

fn type_matches(declared: &str, body: &Value) -> bool {
    matches!(
        (declared, body),
        ("object", Value::Object(_))
            | ("array", Value::Array(_))
            | ("string", Value::String(_))
            | ("number", Value::Number(_))
            | ("integer", Value::Number(_))
            | ("boolean", Value::Bool(_))
            | ("null", Value::Null)
    )
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Object(_) => "object",
        Value::Array(_) => "array",
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Bool(_) => "boolean",
        Value::Null => "null",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn no_type_no_required_accepts_anything() {
        let c = SchemaCheck::default();
        assert!(c.check(&json!({})).is_ok());
        assert!(c.check(&json!("hi")).is_ok());
        assert!(c.check(&Value::Null).is_ok());
    }

    #[test]
    fn type_mismatch_is_rejected() {
        let c = SchemaCheck::from_value(&json!({ "type": "object" }));
        assert!(c.check(&json!({})).is_ok());
        assert!(c.check(&json!([])).is_err());
        assert!(c.check(&json!("nope")).is_err());
    }

    #[test]
    fn missing_required_field_is_rejected() {
        let c = SchemaCheck::from_value(&json!({
            "type": "object",
            "required": ["city", "units"]
        }));
        assert!(c.check(&json!({ "city": "Sydney", "units": "metric" })).is_ok());
        let err = c.check(&json!({ "city": "Sydney" })).unwrap_err();
        assert!(err.contains("units"), "{err}");
    }

    #[test]
    fn required_on_non_object_is_rejected() {
        let c = SchemaCheck::from_value(&json!({
            "type": "object",
            "required": ["x"]
        }));
        assert!(c.check(&json!([])).is_err());
    }

    #[test]
    fn malformed_schema_treated_as_permissive() {
        // The schema is not a JSON object — adapters keep loading so
        // a bad schema file does not prevent the registry from running.
        let c = SchemaCheck::from_value(&json!("not a schema"));
        assert!(c.check(&json!({})).is_ok());
    }
}

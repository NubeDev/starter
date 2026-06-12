//! Validate a connector config against a datasource-kind's JSON Schema.
//!
//! This is the request-time half of the declaration format: the config schema is
//! `additionalProperties: false`, so an unknown field is rejected; declared
//! defaults are applied for absent fields; min/max/pattern bound each value. A
//! config that validates is safe to seal its secret fields and persist.

use serde_json::{Map, Value};

use super::error::DatasourceKindError;
use super::kind::DatasourceKind;

/// Validate `config` against `kind`'s schema (after merging schema defaults) and
/// return the merged config. Rejects unknown fields, missing required fields, and
/// any constraint violation as a 4xx-shaped [`DatasourceKindError::ConfigValidation`].
pub fn validate(kind: &DatasourceKind, config: &Value) -> Result<Value, DatasourceKindError> {
    let merged = apply_defaults(&kind.config_schema, config);
    let validator = jsonschema::validator_for(&kind.config_schema).map_err(|e| {
        // A schema that fails to compile is a pack-author bug; surface it as a
        // validation error rather than panicking on a caller request.
        DatasourceKindError::ConfigValidation {
            kind: kind.name.clone(),
            detail: format!("schema did not compile: {e}"),
        }
    })?;
    if let Err(err) = validator.validate(&merged) {
        return Err(DatasourceKindError::ConfigValidation {
            kind: kind.name.clone(),
            detail: err.to_string(),
        });
    }
    Ok(merged)
}

/// Merge declared schema defaults into the caller's config: a field the caller
/// omitted but the schema gives a `default` for is filled in. Caller-supplied
/// fields win over defaults.
fn apply_defaults(schema: &Value, config: &Value) -> Value {
    let mut out: Map<String, Value> = config.as_object().cloned().unwrap_or_default();
    if let Some(props) = schema.get("properties").and_then(Value::as_object) {
        for (name, spec) in props {
            if out.contains_key(name) {
                continue;
            }
            if let Some(default) = spec.get("default") {
                out.insert(name.clone(), default.clone());
            }
        }
    }
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::super::manifest::{Surface, TestSpec};
    use super::*;
    use serde_json::json;

    fn kind(schema: Value) -> DatasourceKind {
        DatasourceKind {
            name: "test".to_string(),
            surface: Surface::Stream,
            config_schema: schema,
            secret_fields: vec![],
            test: TestSpec::Connect,
            dialect: None,
            description: None,
        }
    }

    #[test]
    fn applies_declared_defaults_for_absent_fields() {
        let k = kind(json!({
            "type": "object",
            "properties": { "port": { "type": "integer", "default": 1883 } },
            "additionalProperties": false
        }));
        let out = validate(&k, &json!({ "host": "x" }));
        // `host` is not declared, so additionalProperties:false rejects it.
        assert!(out.is_err());
        let out = validate(&k, &json!({})).expect("defaults satisfy the schema");
        assert_eq!(out.get("port"), Some(&json!(1883)));
    }

    #[test]
    fn rejects_unknown_field() {
        let k = kind(json!({
            "type": "object",
            "properties": { "host": { "type": "string" } },
            "additionalProperties": false
        }));
        let err = validate(&k, &json!({ "host": "x", "evil": 1 }))
            .expect_err("additionalProperties:false rejects unknown fields");
        assert!(matches!(err, DatasourceKindError::ConfigValidation { .. }));
    }

    #[test]
    fn rejects_out_of_range_value() {
        let k = kind(json!({
            "type": "object",
            "properties": { "qos": { "type": "integer", "minimum": 0, "maximum": 2 } }
        }));
        let err = validate(&k, &json!({ "qos": 9 })).expect_err("maximum bound is enforced");
        assert!(matches!(err, DatasourceKindError::ConfigValidation { .. }));
    }
}

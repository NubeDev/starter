//! Validate caller params against a kind's JSON Schema, then lower them into the
//! binder's [`ParamValue`] map.
//!
//! Validation is the request-time half of the kinds security spine: the schema
//! is `additionalProperties: false`, so an unknown key is rejected; declared
//! defaults are applied for absent keys; min/max/pattern bound each value. Only
//! after a clean validation do values reach the binder — and there each becomes
//! a bound `$N` arg, never inlined. So a value that passes the schema but carries
//! `'); DROP …` is still inert.

use std::collections::BTreeMap;

use nexus_store::{ParamValue, ScalarValue};
use serde_json::{Map, Value};

use super::error::KindError;
use super::kind::QueryKind;

/// Validate `params` against `kind`'s schema (after merging schema defaults) and
/// return the binder param map. Rejects unknown keys, missing required keys, and
/// any constraint violation as a 4xx-shaped [`KindError::ParamValidation`].
pub fn validate(
    kind: &QueryKind,
    params: &Value,
) -> Result<BTreeMap<String, ParamValue>, KindError> {
    let merged = apply_defaults(&kind.params_schema, params);
    let validator = jsonschema::validator_for(&kind.params_schema).map_err(|e| {
        // A schema that fails to compile is a pack-author bug; surface it as a
        // validation error rather than panicking on a caller request.
        KindError::ParamValidation {
            kind: kind.name.clone(),
            detail: format!("schema did not compile: {e}"),
        }
    })?;
    if let Err(err) = validator.validate(&merged) {
        return Err(KindError::ParamValidation {
            kind: kind.name.clone(),
            detail: err.to_string(),
        });
    }
    lower(kind, &merged)
}

/// Merge declared schema defaults into the caller's params: a key the caller
/// omitted but the schema gives a `default` for is filled in, so the SQL's
/// `$param` always binds a value. Caller-supplied keys win over defaults.
fn apply_defaults(schema: &Value, params: &Value) -> Value {
    let mut out: Map<String, Value> = params.as_object().cloned().unwrap_or_default();
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

/// Lower a validated params object into the binder's scalar param map. Every
/// declared property becomes one [`ParamValue`]; the schema already constrained
/// the JSON types, so the lowering is a total mapping over scalars.
fn lower(kind: &QueryKind, merged: &Value) -> Result<BTreeMap<String, ParamValue>, KindError> {
    let obj = merged.as_object().expect("validated params are an object");
    let mut out = BTreeMap::new();
    for (name, value) in obj {
        out.insert(name.clone(), to_scalar(kind, name, value)?);
    }
    Ok(out)
}

/// Map one JSON scalar to a [`ScalarValue`]. A composite value (array/object) is
/// rejected: kind params are scalars the binder binds as single `$N` args (list
/// expansion is a *variable* concern, `$__sqlIn`, not a kind param).
fn to_scalar(kind: &QueryKind, name: &str, value: &Value) -> Result<ScalarValue, KindError> {
    match value {
        Value::String(s) => Ok(ScalarValue::Text(s.clone())),
        Value::Bool(b) => Ok(ScalarValue::Bool(*b)),
        Value::Number(n) if n.is_i64() => Ok(ScalarValue::Int(n.as_i64().unwrap())),
        Value::Number(n) if n.is_u64() => Ok(ScalarValue::Int(n.as_u64().unwrap() as i64)),
        Value::Number(n) => Ok(ScalarValue::Float(n.as_f64().unwrap())),
        other => Err(KindError::ParamValidation {
            kind: kind.name.clone(),
            detail: format!("param `{name}` must be a scalar, got {other}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn kind(schema: Value) -> QueryKind {
        QueryKind {
            name: "nexus.test.k".to_string(),
            sql: String::new(),
            params_schema: schema,
            datasource_kind: "postgres".to_string(),
            tables: vec![],
            datasource_binding: None,
            description: None,
        }
    }

    #[test]
    fn applies_declared_defaults_for_absent_params() {
        let k = kind(json!({
            "type": "object",
            "properties": { "limit": { "type": "integer", "default": 500 } },
            "additionalProperties": false
        }));
        let out = validate(&k, &json!({})).expect("defaults satisfy the schema");
        assert_eq!(out.get("limit"), Some(&ParamValue::Int(500)));
    }

    #[test]
    fn caller_value_wins_over_default() {
        let k = kind(json!({
            "type": "object",
            "properties": { "limit": { "type": "integer", "default": 500 } },
            "additionalProperties": false
        }));
        let out = validate(&k, &json!({ "limit": 25 })).expect("valid override");
        assert_eq!(out.get("limit"), Some(&ParamValue::Int(25)));
    }

    #[test]
    fn rejects_unknown_property() {
        let k = kind(json!({
            "type": "object",
            "properties": { "site_id": { "type": "string" } },
            "additionalProperties": false
        }));
        let err = validate(&k, &json!({ "evil": "x" }))
            .expect_err("additionalProperties:false rejects unknown keys");
        assert!(matches!(err, KindError::ParamValidation { .. }));
    }

    #[test]
    fn rejects_out_of_range_value() {
        let k = kind(json!({
            "type": "object",
            "properties": { "limit": { "type": "integer", "minimum": 1, "maximum": 100 } }
        }));
        let err = validate(&k, &json!({ "limit": 9999 })).expect_err("maximum bound is enforced");
        assert!(matches!(err, KindError::ParamValidation { .. }));
    }

    #[test]
    fn injection_string_passes_schema_but_lowers_to_bound_text() {
        // A value that satisfies the schema but carries SQL still becomes an
        // inert bound Text scalar — the binder binds it as a `$N` arg, so it is
        // never interpreted as SQL.
        let k = kind(json!({
            "type": "object",
            "properties": { "site_id": { "type": "string" } }
        }));
        let out = validate(&k, &json!({ "site_id": "'); DROP TABLE meters; --" }))
            .expect("a plain string is schema-valid");
        assert_eq!(
            out.get("site_id"),
            Some(&ParamValue::Text("'); DROP TABLE meters; --".to_string()))
        );
    }

    #[test]
    fn rejects_composite_param_value() {
        let k = kind(json!({
            "type": "object",
            "properties": { "ids": { "type": "array" } }
        }));
        let err = validate(&k, &json!({ "ids": [1, 2, 3] }))
            .expect_err("array params are not scalars the binder binds");
        assert!(matches!(err, KindError::ParamValidation { .. }));
    }
}

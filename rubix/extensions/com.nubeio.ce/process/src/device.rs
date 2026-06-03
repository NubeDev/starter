//! Device-catalog CRUD over the `ce_devices` warehouse table.
//!
//! A "device" here is a remote control engine the operator can
//! program — identified by `device_id`, carrying its connection
//! details (IP / port / username / password). Writes go through
//! `ctx.warehouse_write()`; the host stamps `tenant_id` and validates
//! columns against the manifest before the row lands.
//!
//! The tool params arrive flat and already schema-validated (see
//! `kinds/device_*_in.json`, all `additionalProperties:false`), so the
//! params object maps straight onto warehouse columns.

use starter_ext_sdk::serde_json::{json, Map, Value};
use starter_ext_sdk::{Error, Row};

use crate::extension::ControlEngineCtx;

/// The host-owned physical table name (manifest name prefixed with
/// `com_nubeio_ce__`).
pub const TABLE: &str = "ce_devices";

/// Extract a required string param or return a validation error.
pub fn take_str(params: &Value, key: &str, tool: &str) -> starter_ext_sdk::Result<String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            Error::Validation(format!(
                "tool {tool}: param `{key}` required, must be string"
            ))
        })
}

/// Build a warehouse row from the flat, schema-validated tool params.
///
/// Empty-string values are dropped so the column's DB default applies
/// and a blank `password` on edit never overwrites the stored secret
/// (the form sends `password: ""` to mean "unchanged"). `tenant_id` is
/// never set here — the host stamps it from the caller.
fn row_from_params(params: &Value) -> Map<String, Value> {
    let mut row = Map::new();
    if let Some(obj) = params.as_object() {
        for (key, value) in obj {
            if matches!(value, Value::String(s) if s.is_empty()) {
                continue;
            }
            row.insert(key.clone(), value.clone());
        }
    }
    row
}

/// `device_create` — insert one engine connection row.
pub fn handle_create(ctx: &ControlEngineCtx, params: &Value) -> starter_ext_sdk::Result<Value> {
    // `device_id` + `ip` are schema-required; assert here for a clear
    // error if a caller bypasses the schema.
    take_str(params, "device_id", "device_create")?;
    take_str(params, "ip", "device_create")?;

    let row = row_from_params(params);
    let affected = ctx.warehouse_write().insert(TABLE, vec![Row::from_map(row)])?;
    Ok(json!({ "operation": "create", "affected": affected }))
}

/// `device_update` — set columns on one engine row (keyed by
/// `device_id`).
pub fn handle_update(ctx: &ControlEngineCtx, params: &Value) -> starter_ext_sdk::Result<Value> {
    take_str(params, "device_id", "device_update")?;

    let row = row_from_params(params);
    let affected = ctx
        .warehouse_write()
        .update(TABLE, "device_id", vec![Row::from_map(row)])?;
    Ok(json!({ "operation": "update", "affected": affected }))
}

/// `device_delete` — remove one engine row by `device_id`.
pub fn handle_delete(ctx: &ControlEngineCtx, params: &Value) -> starter_ext_sdk::Result<Value> {
    let device_id = take_str(params, "device_id", "device_delete")?;
    let affected = ctx
        .warehouse_write()
        .delete(TABLE, "device_id", vec![json!(device_id)])?;
    Ok(json!({ "operation": "delete", "affected": affected }))
}

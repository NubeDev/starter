//! Device-catalog CRUD over the `ce_devices` warehouse table.
//!
//! A "device" here is a remote control engine the operator can
//! program — identified by `device_id`, carrying its connection
//! details (IP / port / username / password). Writes go through
//! `ctx.warehouse_write()`; the host stamps `tenant_id` and validates
//! columns against the manifest before the row lands.
//!
//! SCAFFOLD: the row-building / write calls are sketched and marked
//! `TODO`. Wire them to `ctx.warehouse_write().insert/update/delete`
//! once the column shape is final.

use starter_ext_sdk::serde_json::{json, Value};
use starter_ext_sdk::Error;

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

/// `device_create` — insert one engine connection row.
pub fn handle_create(_ctx: &ControlEngineCtx, params: &Value) -> starter_ext_sdk::Result<Value> {
    let device_id = take_str(params, "device_id", "device_create")?;
    let _ip = take_str(params, "ip", "device_create")?;

    // TODO: build the row from `params` (name, description, engine_kind,
    // ip, port, username, password, status) and insert it:
    //
    //   let row = Row::from_map(...);
    //   let affected = ctx.warehouse_write().insert(TABLE, vec![row])?;
    //
    // Remember NOT to set `tenant_id` — the host stamps it.
    let _ = device_id;
    Ok(json!({ "operation": "create", "affected": 0 }))
}

/// `device_update` — set columns on one engine row (keyed by
/// `device_id`).
pub fn handle_update(_ctx: &ControlEngineCtx, params: &Value) -> starter_ext_sdk::Result<Value> {
    let device_id = take_str(params, "device_id", "device_update")?;

    // TODO: build a partial row from the remaining `params` keys and:
    //
    //   let affected = ctx.warehouse_write().update(TABLE, "device_id", vec![row])?;
    let _ = device_id;
    Ok(json!({ "operation": "update", "affected": 0 }))
}

/// `device_delete` — remove one engine row by `device_id`.
pub fn handle_delete(_ctx: &ControlEngineCtx, params: &Value) -> starter_ext_sdk::Result<Value> {
    let device_id = take_str(params, "device_id", "device_delete")?;

    // TODO: delete the row:
    //
    //   let affected = ctx.warehouse_write()
    //       .delete(TABLE, "device_id", vec![json!(device_id)])?;
    let _ = device_id;
    Ok(json!({ "operation": "delete", "affected": 0 }))
}

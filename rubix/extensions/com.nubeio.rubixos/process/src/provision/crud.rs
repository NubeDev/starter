//! Shared input-extraction and thin CRUD wrappers over
//! `ctx.warehouse_write()`, mirroring `com.rubix.geo`'s `crud.rs`.
//!
//! Every provisioning write funnels through here so the `tenant_id`
//! stamp, column validation and error shape stay uniform. The host
//! validates columns against the manifest and stamps `tenant_id`; the
//! extension never writes it.

use starter_ext_sdk::serde_json::{json, Map, Value};
use starter_ext_sdk::{Error, Row};

use crate::provision::RubixOsCtx;

/// Pull a required string field from the tool params.
pub fn take_str(params: &Value, key: &str, tool: &str) -> starter_ext_sdk::Result<String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Error::Validation(format!("{tool}: `{key}` (string) is required")))
}

/// Pull the `row` object from the tool params.
pub fn take_row(params: &Value, tool: &str) -> starter_ext_sdk::Result<Map<String, Value>> {
    params
        .get("row")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| Error::Validation(format!("{tool}: `row` (object) is required")))
}

/// Insert one row, requiring `key_col` to be a non-empty string so a
/// row never lands without its primary key.
pub fn crud_insert(
    ctx: &RubixOsCtx,
    params: &Value,
    table: &str,
    key_col: &str,
    tool: &str,
) -> starter_ext_sdk::Result<Value> {
    let row = take_row(params, tool)?;
    require_key(&row, key_col, tool)?;
    let affected = ctx.warehouse_write().insert(table, vec![Row::from_map(row)])?;
    Ok(json!({ "operation": "create", "affected": affected }))
}

/// Update one row by `key_col`, requiring the key to be present.
pub fn crud_update(
    ctx: &RubixOsCtx,
    params: &Value,
    table: &str,
    key_col: &str,
    tool: &str,
) -> starter_ext_sdk::Result<Value> {
    let row = take_row(params, tool)?;
    require_key(&row, key_col, tool)?;
    let affected = ctx
        .warehouse_write()
        .update(table, key_col, vec![Row::from_map(row)])?;
    Ok(json!({ "operation": "update", "affected": affected }))
}

/// Extract and validate a non-empty array of string ids.
pub fn take_id_list(
    params: &Value,
    ids_field: &str,
    tool: &str,
) -> starter_ext_sdk::Result<Vec<Value>> {
    let ids = params
        .get(ids_field)
        .and_then(Value::as_array)
        .ok_or_else(|| {
            Error::Validation(format!("{tool}: `{ids_field}` (array of strings) is required"))
        })?;
    if ids.is_empty() {
        return Err(Error::Validation(format!(
            "{tool}: `{ids_field}` must not be empty"
        )));
    }
    ids.iter()
        .map(|v| match v.as_str() {
            Some(s) if !s.is_empty() => Ok(Value::String(s.to_owned())),
            _ => Err(Error::Validation(format!(
                "{tool}: every entry in `{ids_field}` must be a non-empty string"
            ))),
        })
        .collect()
}

fn require_key(row: &Map<String, Value>, key_col: &str, tool: &str) -> starter_ext_sdk::Result<()> {
    let ok = row
        .get(key_col)
        .and_then(Value::as_str)
        .is_some_and(|s| !s.is_empty());
    if ok {
        Ok(())
    } else {
        Err(Error::Validation(format!(
            "{tool}: `row.{key_col}` (non-empty string) is required"
        )))
    }
}

use starter_ext_sdk::serde_json::{json, Map, Value};
use starter_ext_sdk::Row;

use crate::extension::GeoCtx;

pub fn take_str(params: &Value, key: &str, tool: &str) -> starter_ext_sdk::Result<String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            starter_ext_sdk::Error::Validation(format!("{tool}: `{key}` (string) is required"))
        })
}

pub fn take_row(params: &Value, tool: &str) -> starter_ext_sdk::Result<Map<String, Value>> {
    params
        .get("row")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| {
            starter_ext_sdk::Error::Validation(format!("{tool}: `row` (object) is required"))
        })
}

pub fn crud_insert(
    ctx: &GeoCtx,
    params: &Value,
    table: &str,
    tool: &str,
) -> starter_ext_sdk::Result<Value> {
    let row = take_row(params, tool)?;
    let affected = ctx.warehouse_write().insert(table, vec![Row::from_map(row)])?;
    Ok(json!({ "operation": "create", "affected": affected }))
}

pub fn crud_update(
    ctx: &GeoCtx,
    params: &Value,
    table: &str,
    key_col: &str,
    tool: &str,
) -> starter_ext_sdk::Result<Value> {
    let row = take_row(params, tool)?;
    let has_key = row
        .get(key_col)
        .and_then(Value::as_str)
        .is_some_and(|s| !s.is_empty());
    if !has_key {
        return Err(starter_ext_sdk::Error::Validation(format!(
            "{tool}: `row.{key_col}` (non-empty string) is required"
        )));
    }
    let affected = ctx
        .warehouse_write()
        .update(table, key_col, vec![Row::from_map(row)])?;
    Ok(json!({ "operation": "update", "affected": affected }))
}

pub fn crud_delete(
    ctx: &GeoCtx,
    params: &Value,
    table: &str,
    key_col: &str,
    ids_field: &str,
    tool: &str,
) -> starter_ext_sdk::Result<Value> {
    let ids = params
        .get(ids_field)
        .and_then(Value::as_array)
        .ok_or_else(|| {
            starter_ext_sdk::Error::Validation(format!(
                "{tool}: `{ids_field}` (array of strings) is required"
            ))
        })?;
    if ids.is_empty() {
        return Err(starter_ext_sdk::Error::Validation(format!(
            "{tool}: `{ids_field}` must not be empty"
        )));
    }
    let keys: Vec<Value> = ids
        .iter()
        .map(|v| match v.as_str() {
            Some(s) if !s.is_empty() => Ok(Value::String(s.to_owned())),
            _ => Err(starter_ext_sdk::Error::Validation(format!(
                "{tool}: every entry in `{ids_field}` must be a non-empty string"
            ))),
        })
        .collect::<starter_ext_sdk::Result<_>>()?;
    let affected = ctx.warehouse_write().delete(table, key_col, keys)?;
    Ok(json!({ "operation": "delete", "affected": affected }))
}

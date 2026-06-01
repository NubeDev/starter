//! Resolve the site / location / page a device is placed at, creating
//! location and page on demand from the scan flow (BARCODE.md §5.1
//! step 2). Sites are picked, never auto-created here — site creation
//! is its own explicit tool (`bc_site_create`).
//!
//! Each create-on-demand mints a deterministic id from the name so a
//! re-scan with the same `new_location`/`new_page` name does not spawn
//! a duplicate (idempotency, §7).

use starter_ext_sdk::serde_json::{json, Map, Value};
use starter_ext_sdk::{Error, Row};

use crate::provision::device::Placement;
use crate::provision::ids::slug_id;
use crate::provision::RubixOsCtx;

/// Resolve placement from the provision params, performing any
/// create-on-demand inserts and returning the chosen ids.
pub fn resolve(ctx: &RubixOsCtx, params: &Value) -> starter_ext_sdk::Result<Placement> {
    let site_id = opt_str(params, "site_id");
    let location_id = resolve_location(ctx, params, site_id.as_deref())?;
    let page_id = resolve_page(ctx, params)?;
    Ok(Placement {
        site_id,
        location_id,
        page_id,
        name: opt_str(params, "name"),
    })
}

/// Either the explicit `location_id`, or a freshly-created location
/// from `new_location: { name }`.
fn resolve_location(
    ctx: &RubixOsCtx,
    params: &Value,
    site_id: Option<&str>,
) -> starter_ext_sdk::Result<Option<String>> {
    if let Some(id) = opt_str(params, "location_id") {
        return Ok(Some(id));
    }
    let Some(name) = new_name(params, "new_location")? else {
        return Ok(None);
    };
    let location_id = slug_id("loc", &name);
    let mut row = Map::new();
    row.insert("location_id".into(), json!(location_id));
    row.insert("site_id".into(), json!(site_id));
    row.insert("name".into(), json!(name));
    ctx.warehouse_write()
        .insert("bc_locations", vec![Row::from_map(row)])?;
    Ok(Some(location_id))
}

/// Either the explicit `page_id`, or a freshly-created page from
/// `new_page: { name }`.
fn resolve_page(ctx: &RubixOsCtx, params: &Value) -> starter_ext_sdk::Result<Option<String>> {
    if let Some(id) = opt_str(params, "page_id") {
        return Ok(Some(id));
    }
    let Some(name) = new_name(params, "new_page")? else {
        return Ok(None);
    };
    let page_id = slug_id("page", &name);
    let mut row = Map::new();
    row.insert("page_id".into(), json!(page_id));
    row.insert("name".into(), json!(name));
    ctx.warehouse_write()
        .insert("bc_pages", vec![Row::from_map(row)])?;
    Ok(Some(page_id))
}

/// Read `{ name }` out of a `new_location` / `new_page` object.
fn new_name(params: &Value, key: &str) -> starter_ext_sdk::Result<Option<String>> {
    let Some(obj) = params.get(key) else {
        return Ok(None);
    };
    let name = obj
        .get("name")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| Error::Validation(format!("bc_provision: `{key}.name` must be non-empty")))?;
    Ok(Some(name.to_owned()))
}

fn opt_str(params: &Value, key: &str) -> Option<String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

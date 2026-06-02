//! `bc_device_assign_page` — place an already-commissioned device on a
//! dashboard page after the fact.
//!
//! Provisioning is page-optional (BARCODE.md §6.2): a device scanned
//! without a page lands `status = pending` with its points + alarms
//! written but no widgets. This verb closes that gap — given a
//! `device_id` and a page (existing `page_id` or `new_page: { name }`),
//! it generates the device's widgets on that page, stamps the device's
//! `page_id` + `site_id`, and flips `status` to `provisioned`.
//!
//! Idempotent: widgets are replaced (cleared then re-inserted) so a
//! re-assign repairs rather than dupes, matching the orchestrate
//! re-scan semantics (§7).

use starter_ext_sdk::serde_json::{json, Map, Value};
use starter_ext_sdk::{Error, Row};

use crate::provision::crud::take_str;
use crate::provision::placement;
use crate::provision::points::{self, Toggles};
use crate::provision::{template_store, widgets};
use crate::provision::RubixOsCtx;

/// Run the assign-page flow and return
/// `{ device_id, page_id, widgets, status }`.
pub fn handle(ctx: &RubixOsCtx, params: &Value) -> starter_ext_sdk::Result<Value> {
    let device_id = take_str(params, "device_id", "bc_device_assign_page")?;
    let device = load_device(ctx, &device_id)?;

    // The page may exist or be created on demand from `new_page`. A new
    // page inherits the device's site and location so it slots into the
    // "Site → Location → its pages" tree.
    let page_id = placement::resolve_page_only(
        ctx,
        params,
        device.site_id.as_deref(),
        device.location_id.as_deref(),
    )?
    .ok_or_else(|| {
        Error::Validation("bc_device_assign_page: a `page_id` or `new_page.name` is required".into())
    })?;

    // Re-derive the device's points from its template so widget slots
    // and titles match what a fresh provision would produce. Toggles are
    // left at template defaults here — assigning a page changes display,
    // not trend/alarm policy (use bc_device_update for those).
    let template = template_store::load(ctx, &device.template)?;
    let resolved = points::resolve(&device_id, &template.expanded_points(), Toggles::default());

    let rows = widgets::rows_for(&page_id, &device_id, &device.name, &template, &resolved);
    let widget_count = rows.len();
    replace_widgets(ctx, &device_id, rows)?;

    // Stamp the page (and its site) onto the device and mark it live.
    stamp_device(ctx, &device_id, &page_id, device.site_id.as_deref())?;

    Ok(json!({
        "device_id": device_id,
        "page_id": page_id,
        "widgets": widget_count,
        "status": "provisioned",
    }))
}

/// The device fields this verb needs: its template (to regenerate
/// widgets), display name, and current site/location (an on-demand page
/// inherits them).
struct DeviceRef {
    template: String,
    name: String,
    site_id: Option<String>,
    location_id: Option<String>,
}

/// Read one device row by id via the bundled list template, matching on
/// `device_id` in Rust (the list template has no by-id variant).
fn load_device(ctx: &RubixOsCtx, device_id: &str) -> starter_ext_sdk::Result<DeviceRef> {
    let rows = ctx.warehouse_read().query(
        "com.nubeio.rubixos.bc_devices_list",
        json!({ "site_id": "", "status": "", "limit": 1000 }),
    )?;
    let row = rows
        .iter()
        .find(|r| r.0.get("device_id").and_then(Value::as_str) == Some(device_id))
        .ok_or_else(|| {
            Error::Validation(format!(
                "bc_device_assign_page: unknown device `{device_id}`"
            ))
        })?;
    let template = row
        .0
        .get("template")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            Error::Validation(format!("bc_device_assign_page: device `{device_id}` has no template"))
        })?
        .to_owned();
    let name = row
        .0
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(device_id)
        .to_owned();
    let site_id = row
        .0
        .get("site_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    let location_id = row
        .0
        .get("location_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    Ok(DeviceRef { template, name, site_id, location_id })
}

/// Clear this device's existing widgets then insert the fresh set —
/// the repair semantics a re-assign relies on (keyed by `device_id`).
fn replace_widgets(ctx: &RubixOsCtx, device_id: &str, rows: Vec<Row>) -> starter_ext_sdk::Result<()> {
    ctx.warehouse_write()
        .delete("bc_widgets", "device_id", vec![Value::String(device_id.to_owned())])?;
    if !rows.is_empty() {
        ctx.warehouse_write().insert("bc_widgets", rows)?;
    }
    Ok(())
}

/// Patch the device row: set `page_id`, carry the page's `site_id` (so
/// the device and its page share a site), and flip `status` to live.
fn stamp_device(
    ctx: &RubixOsCtx,
    device_id: &str,
    page_id: &str,
    site_id: Option<&str>,
) -> starter_ext_sdk::Result<()> {
    let mut row = Map::new();
    row.insert("device_id".into(), json!(device_id));
    row.insert("page_id".into(), json!(page_id));
    if let Some(sid) = site_id {
        row.insert("site_id".into(), json!(sid));
    }
    row.insert("status".into(), json!("provisioned"));
    ctx.warehouse_write()
        .update("bc_devices", "device_id", vec![Row::from_map(row)])?;
    Ok(())
}

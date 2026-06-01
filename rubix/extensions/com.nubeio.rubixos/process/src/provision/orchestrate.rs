//! `bc_provision` — the orchestrator (BARCODE.md §5.1).
//!
//! Decode → load template → place → write device + points + alarms +
//! widgets + audit log, in FK-safe order. Everything downstream of
//! identity is deterministic and template-driven; the only human
//! inputs are placement (site/location/page) and the trend/alarm
//! toggles.
//!
//! **Idempotency.** There is no extension-facing transaction in the
//! SPI yet (BARCODE.md §10 Q1), so provision is best-effort
//! multi-insert made safe by being re-runnable: a re-scan of the same
//! `device_id` clears that device's child rows (points/widgets/alarms)
//! and re-writes them, and upserts the device row — so a half-written
//! provision is repaired by scanning again rather than duplicated.

use starter_ext_sdk::serde_json::{json, Value};
use starter_ext_sdk::Row;

use crate::provision::crud::take_str;
use crate::provision::ids::event_id;
use crate::provision::{alarms, decode, device, placement, points, template_store, widgets};
use crate::provision::RubixOsCtx;

/// Run a full provision and return the summary
/// `{ device_id, points, widgets, alarms, page_id, warnings }`.
pub fn handle(ctx: &RubixOsCtx, params: &Value) -> starter_ext_sdk::Result<Value> {
    let barcode = take_str(params, "barcode", "bc_provision")?;
    let mut warnings: Vec<String> = Vec::new();

    // 1. Decode + resolve template.
    let identity = decode::decode(&barcode)?;
    let template = template_store::load(ctx, &identity.model)?;

    // 2. Placement (create location/page on demand).
    let placement = placement::resolve(ctx, params)?;
    let page_id = placement.page_id.clone();
    if page_id.is_none() {
        warnings.push("no page selected — widgets were not generated".into());
    }

    // 3. Device row (upsert so a re-scan repairs rather than dupes).
    let device_row = device::build_row(&identity, &template.template, &placement);
    upsert_device(ctx, &identity.id, device_row)?;
    log_step(ctx, &identity.id, "device", 0, "device row written");

    // 4. Points — clear this device's existing points then re-insert.
    let toggles = read_toggles(params);
    let resolved = points::resolve(&identity.id, &template.expanded_points(), toggles);
    replace_children(ctx, "bc_points", &identity.id, point_rows(&resolved))?;
    log_step(ctx, &identity.id, "points", 1, &format!("{} points", resolved.len()));

    // 5. Alarms — materialise armed rules.
    let alarm_rows = alarms::rows_for(&identity.id, &resolved);
    let alarm_count = alarm_rows.len();
    replace_children(ctx, "bc_alarms", &identity.id, alarm_rows)?;
    log_step(ctx, &identity.id, "alarms", 2, &format!("{alarm_count} alarms"));

    // 6. Widgets — only when placed on a page.
    let widget_count = match &page_id {
        Some(pid) => {
            let device_name = placement.name.clone().unwrap_or_else(|| identity.id.clone());
            let rows = widgets::rows_for(pid, &identity.id, &device_name, &template, &resolved);
            let n = rows.len();
            replace_children(ctx, "bc_widgets", &identity.id, rows)?;
            n
        }
        None => 0,
    };
    log_step(ctx, &identity.id, "widgets", 3, &format!("{widget_count} widgets"));

    // 7. Final audit row.
    log_step(ctx, &identity.id, "complete", 4, "provision complete");

    Ok(json!({
        "device_id": identity.id,
        "points": resolved.len(),
        "widgets": widget_count,
        "alarms": alarm_count,
        "page_id": page_id,
        "warnings": warnings,
    }))
}

/// Read the master trend/alarm toggles from the request.
fn read_toggles(params: &Value) -> points::Toggles {
    points::Toggles {
        trend: params.get("trend").and_then(Value::as_bool),
        alarm: params.get("alarm").and_then(Value::as_bool),
    }
}

fn point_rows(resolved: &[points::ResolvedPoint]) -> Vec<Row> {
    resolved.iter().map(points::ResolvedPoint::row).collect()
}

/// Upsert the device row: update by `device_id`, insert if new.
fn upsert_device(ctx: &RubixOsCtx, device_id: &str, row: Row) -> starter_ext_sdk::Result<()> {
    let updated = ctx
        .warehouse_write()
        .update("bc_devices", "device_id", vec![row.clone()])?;
    if updated == 0 {
        ctx.warehouse_write().insert("bc_devices", vec![row])?;
    }
    let _ = device_id;
    Ok(())
}

/// Clear a device's existing child rows in `table`, then insert the
/// fresh set — the repair semantics a re-scan relies on.
fn replace_children(
    ctx: &RubixOsCtx,
    table: &str,
    device_id: &str,
    rows: Vec<Row>,
) -> starter_ext_sdk::Result<()> {
    ctx.warehouse_write()
        .delete(table, "device_id", vec![Value::String(device_id.to_owned())])?;
    if !rows.is_empty() {
        ctx.warehouse_write().insert(table, rows)?;
    }
    Ok(())
}

/// Append a best-effort `bc_provision_log` audit row. A logging
/// failure must not fail the provision, so the error is swallowed
/// after a stderr note.
fn log_step(ctx: &RubixOsCtx, device_id: &str, step: &str, seq: u32, detail: &str) {
    let mut row = starter_ext_sdk::serde_json::Map::new();
    row.insert("event_id".into(), json!(event_id(device_id, step, seq)));
    row.insert("device_id".into(), json!(device_id));
    row.insert("event".into(), json!("provision"));
    row.insert("step".into(), json!(step));
    row.insert("detail".into(), json!(detail));
    if let Err(e) = ctx
        .warehouse_write()
        .insert("bc_provision_log", vec![Row::from_map(row)])
    {
        eprintln!("bc_provision: audit log insert failed (non-fatal): {e}");
    }
}

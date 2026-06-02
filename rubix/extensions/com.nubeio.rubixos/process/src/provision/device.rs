//! Build, update and decommission `bc_devices` rows.
//!
//! Create is part of the `bc_provision` orchestration (one device row
//! per scan). Update is a plain row patch by `device_id`.
//! Decommission is soft by default (`status = decommissioned`,
//! history retained) or hard (cascade-delete the device's points,
//! widgets and alarms) when `hard: true` (BARCODE.md §7).

use starter_ext_sdk::serde_json::{json, Map, Value};
use starter_ext_sdk::{Error, Row};

use crate::provision::crud::{crud_update, take_id_list};
use crate::provision::identity::ScannedIdentity;
use crate::provision::RubixOsCtx;

/// Placement chosen for a device at provision time.
pub struct Placement {
    pub site_id: Option<String>,
    pub location_id: Option<String>,
    pub page_id: Option<String>,
    pub name: Option<String>,
}

/// Build the single `bc_devices` row for a freshly-scanned device.
pub fn build_row(identity: &ScannedIdentity, template: &str, placement: &Placement) -> Row {
    let name = placement
        .name
        .clone()
        .unwrap_or_else(|| identity.id.clone());
    let mut row = Map::new();
    row.insert("device_id".into(), json!(identity.id));
    row.insert("template".into(), json!(template));
    row.insert("name".into(), json!(name));
    row.insert("network".into(), json!(identity.network));
    row.insert("address".into(), json!(identity.address));
    row.insert("default_ip".into(), json!(identity.default_ip));
    row.insert("hw_rev".into(), json!(identity.hw));
    row.insert("site_id".into(), json!(placement.site_id));
    row.insert("location_id".into(), json!(placement.location_id));
    row.insert("page_id".into(), json!(placement.page_id));
    // A device with no page is commissioned but not yet displayed — it
    // sits `pending` until a page is assigned (then widgets generate and
    // it flips to `provisioned`). Page is therefore optional at provision.
    let status = if placement.page_id.is_some() { "provisioned" } else { "pending" };
    row.insert("status".into(), json!(status));
    Row::from_map(row)
}

/// `bc_device_update` — rename, re-place, change toggles via a row
/// patch keyed on `device_id`.
pub fn handle_update(ctx: &RubixOsCtx, params: &Value) -> starter_ext_sdk::Result<Value> {
    crud_update(ctx, params, "bc_devices", "device_id", "bc_device_update")
}

/// `bc_device_decommission` — soft status flip, or hard cascade.
pub fn handle_decommission(ctx: &RubixOsCtx, params: &Value) -> starter_ext_sdk::Result<Value> {
    let tool = "bc_device_decommission";
    let keys = take_id_list(params, "device_ids", tool)?;
    let hard = params.get("hard").and_then(Value::as_bool).unwrap_or(false);

    if !hard {
        // Soft: flip status on each device, leaving points/history.
        let rows: Vec<Row> = keys
            .iter()
            .filter_map(Value::as_str)
            .map(|id| {
                let mut row = Map::new();
                row.insert("device_id".into(), json!(id));
                row.insert("status".into(), json!("decommissioned"));
                Row::from_map(row)
            })
            .collect();
        let affected = ctx
            .warehouse_write()
            .update("bc_devices", "device_id", rows)?;
        return Ok(json!({ "operation": "decommission", "mode": "soft", "affected": affected }));
    }

    // Hard: cascade child rows first, then the devices themselves, so
    // a failure can't orphan points/widgets/alarms behind a deleted
    // device. Children are keyed by device_id.
    let write = ctx.warehouse_write();
    let mut child_deleted = 0u64;
    for table in ["bc_points", "bc_widgets", "bc_alarms"] {
        child_deleted += write.delete(table, "device_id", keys.clone())?;
    }
    let devices_deleted = write.delete("bc_devices", "device_id", keys)?;
    if devices_deleted == 0 {
        return Err(Error::Validation(format!(
            "{tool}: no matching devices to delete"
        )));
    }
    Ok(json!({
        "operation": "decommission",
        "mode": "hard",
        "affected": devices_deleted,
        "children_deleted": child_deleted,
    }))
}

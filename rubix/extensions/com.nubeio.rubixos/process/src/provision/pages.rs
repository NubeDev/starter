//! `bc_page_update` / `bc_page_delete` — page mutation beyond create.
//!
//! Create lives in [`crate::provision::handle_page_create`] (a thin
//! `crud_insert`). Update is a thin `crud_update` keyed by `page_id`.
//! Delete is the one that needs care: a page owns its dashboard widgets
//! and may have devices placed on it, so a naive row delete would leave
//! dangling widgets and devices pointing at a page that no longer
//! exists.
//!
//! Delete therefore, in FK-safe order:
//!   1. clears the page's `bc_widgets` (keyed by `page_id`),
//!   2. detaches any devices placed on the page — nulls their `page_id`
//!      and flips `status` back to `pending` so they resurface as
//!      unprovisioned and can be re-placed,
//!   3. deletes the page row.
//!
//! Step 2 keeps the device + its points/alarms/history intact; only its
//! placement is undone. This mirrors the "page optional at provision"
//! model in [`crate::provision::device`]: a device with no page is
//! `pending`.

use starter_ext_sdk::serde_json::{json, Map, Value};
use starter_ext_sdk::{Error, Row};

use crate::provision::crud::take_row;
use crate::provision::RubixOsCtx;

/// `bc_page_update` — update a page row by `page_id` (rename, or re-pin
/// its `site_id` / `location_id`). Requires `row.page_id`.
pub fn handle_update(ctx: &RubixOsCtx, params: &Value) -> starter_ext_sdk::Result<Value> {
    crate::provision::crud::crud_update(ctx, params, "bc_pages", "page_id", "bc_page_update")
}

/// `bc_page_delete` — delete a page, cleaning up its widgets and
/// detaching (not deleting) the devices placed on it.
pub fn handle_delete(ctx: &RubixOsCtx, params: &Value) -> starter_ext_sdk::Result<Value> {
    let tool = "bc_page_delete";
    let row = take_row(params, tool)?;
    let page_id = require_page_id(&row, tool)?;
    let key = vec![Value::String(page_id.clone())];

    let write = ctx.warehouse_write();

    // 1. Drop the page's dashboard widgets.
    let widgets_deleted = write.delete("bc_widgets", "page_id", key.clone())?;

    // 2. Detach devices placed here: null `page_id`, flip back to
    //    `pending` so they surface as unprovisioned and re-placeable.
    //    We `update` by `page_id` (the column we're clearing is also the
    //    match key, which is fine — the write backend reads the key
    //    before applying the SET).
    let devices_detached = detach_devices(ctx, &page_id)?;

    // 3. Delete the page row itself.
    let pages_deleted = write.delete("bc_pages", "page_id", key)?;
    if pages_deleted == 0 {
        return Err(Error::Validation(format!(
            "{tool}: no matching page to delete"
        )));
    }

    Ok(json!({
        "operation": "delete",
        "affected": pages_deleted,
        "widgets_deleted": widgets_deleted,
        "devices_detached": devices_detached,
    }))
}

/// Null `page_id` + set `status = pending` on every device placed on
/// `page_id`. Returns the number of devices detached.
fn detach_devices(ctx: &RubixOsCtx, page_id: &str) -> starter_ext_sdk::Result<u64> {
    // Find the devices currently on this page so we can update them by
    // their own primary key (`device_id`) — clearing `page_id` while
    // matching on it would otherwise be ambiguous across write backends.
    let rows = ctx.warehouse_read().query(
        "com.nubeio.rubixos.bc_devices_list",
        json!({ "site_id": "", "status": "", "limit": 1000 }),
    )?;
    let mut detached = 0u64;
    let write = ctx.warehouse_write();
    for r in rows {
        let on_page = r
            .0
            .get("page_id")
            .and_then(Value::as_str)
            .is_some_and(|p| p == page_id);
        if !on_page {
            continue;
        }
        let Some(device_id) = r.0.get("device_id").and_then(Value::as_str) else {
            continue;
        };
        let mut update: Map<String, Value> = Map::new();
        update.insert("device_id".into(), json!(device_id));
        update.insert("page_id".into(), Value::Null);
        update.insert("status".into(), json!("pending"));
        detached += write.update("bc_devices", "device_id", vec![Row::from_map(update)])?;
    }
    Ok(detached)
}

fn require_page_id(row: &Map<String, Value>, tool: &str) -> starter_ext_sdk::Result<String> {
    row.get("page_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| Error::Validation(format!("{tool}: `row.page_id` (non-empty string) is required")))
}

//! `com.acme.devices` — process flavour.
//!
//! A real supervised child: the host spawns this binary, frames stdio
//! JSON-RPC against it, health-checks it, restarts it per the manifest's
//! `supervision:` policy, and reports live pid/uptime/RSS/CPU.
//!
//! It serves the Setup / Automation Builder's two **domain side-effect node
//! kinds** (DOCS §9): `com.acme.device.create` and `com.acme.sensor.register`.
//! Both are **idempotent on a natural key** (DOCS §8c) so resume re-entry after
//! a failed step never double-provisions hardware. Idempotency here is achieved
//! the simplest robust way: the output id is a *pure function* of the natural
//! key (a stable hash of the barcode / device_id), so the same input always
//! yields the same id with no shared mutable state to corrupt across restarts.
//!
//! Identity (`caller_*`) arrives on the input from the host's **server-seeded
//! trusted slots** (DOCS §9), never from client form input — the node just
//! reads it like any other field.

use starter_ext_sdk::serde_json::{json, Value};
use starter_ext_sdk::Extension;

/// The extension's unit struct (SCOPE R5: no fields — state lives in Ctx).
#[derive(Extension)]
#[extension(manifest = "../block.yaml")]
pub struct Devices;

starter_ext_sdk::requires! {
    name = DevicesCtx,
    // WS-17: `warehouse_write` grants `ctx.warehouse_write()` — the
    // tenant-stamped persist path into the extension's own `devices` table.
    capabilities = [warehouse_write],
}

/// A stable, collision-resistant-enough id derived purely from a natural key.
/// Same key → same id, which is what makes the side effect idempotent (DOCS
/// §8c) without any persisted dedup table for this example.
fn stable_id(prefix: &str, key: &str) -> String {
    // FNV-1a 64-bit — deterministic, dependency-free, good enough to demo a
    // natural-key → stable-id mapping.
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in key.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{prefix}-{hash:016x}")
}

fn require_str<'a>(params: &'a Value, field: &str) -> starter_ext_sdk::Result<&'a str> {
    params
        .get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| starter_ext_sdk::Error::extension_internal(format!("missing `{field}`")))
}

impl DevicesToolHandlers for Devices {
    type Ctx = DevicesCtx;

    /// `com.acme.device.create` — provision a device from a scanned barcode.
    /// Idempotent on `barcode`: the same barcode always returns the same
    /// `device_id`, so resume re-entry creates no second device (DOCS §8c).
    fn handle_com_acme_devices_device_create(
        &self,
        ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        let barcode = require_str(&params, "barcode")?;
        // Identity is read from the server-seeded trusted slots, never the
        // form (DOCS §9). Optional here — used for the row's `owner`/`team`.
        let owner = params
            .get("caller_user_id")
            .and_then(|v| v.as_str())
            .unwrap_or("system");
        // The caller's first team scopes the row for the read kind's
        // `$caller_team_ids` filter (P3a). Empty ⇒ visible to the whole tenant.
        let team = params
            .get("caller_team_ids")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let location = params.get("location").and_then(|v| v.as_str()).unwrap_or("");
        let device_id = stable_id("dev", barcode);

        // WS-17: PERSIST the device into the extension's own table. The host
        // stamps `tenant_id` from the caller and upserts on `device_id` (the
        // table's natural key), so re-running the same barcode updates the row
        // in place rather than creating a duplicate (DOCS §8c idempotency, now
        // proven against the table, not just a derived id).
        // `created_at` is omitted — the column's `DEFAULT now()` stamps it
        // server-side, so the extension needs no clock dependency.
        let row = match json!({
            "device_id": device_id,
            "barcode": barcode,
            "location": location,
            "owner": owner,
            "team": team,
        }) {
            Value::Object(map) => starter_ext_sdk::Row::from_map(map),
            _ => unreachable!("json! object literal is always an object"),
        };
        let inserted = ctx.warehouse_write().insert("devices", vec![row])?;

        Ok(json!({
            "device_id": device_id,
            "persisted": inserted,
            "out": format!("device {device_id} provisioned for {owner} (barcode {barcode})"),
        }))
    }

    /// `com.acme.sensor.register` — register a sensor against the upstream
    /// device. Idempotent on `device_id`.
    fn handle_com_acme_devices_sensor_register(
        &self,
        _ctx: &Self::Ctx,
        params: Value,
    ) -> starter_ext_sdk::Result<Value> {
        let device_id = require_str(&params, "device_id")?;
        let sensor_id = stable_id("sen", device_id);
        Ok(json!({
            "sensor_id": sensor_id,
            "out": format!("sensor {sensor_id} registered on {device_id}"),
        }))
    }
}

// Emits `pub async fn run() -> starter_ext_sdk::Result<()>` driving the stdio
// JSON-RPC loop the supervisor speaks to.
starter_ext_sdk::register_process_main! {
    extension: Devices,
    ctx: DevicesCtx,
    instance: Devices,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("acme-devices-extension exiting with error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

//! Control-engine REST proxy.
//!
//! The extension process is the bridge between the browser and a
//! remote flow-based runtime (Niagara-kit controller, Sedona device,
//! …). A handler:
//!
//!   1. Looks up the engine's connection row in `ce_devices` by
//!      `device_id` (via `ctx.warehouse_read()`).
//!   2. Builds an HTTP request to that engine's REST API using the
//!      stored IP / port / credentials.
//!   3. Forwards the call and returns the engine's response shaped
//!      for the wiresheet UI.
//!
//! Keeping the credentials and the HTTP egress server-side means the
//! browser never holds the engine password and the call is subject to
//! the host's capability gating.
//!
//! SCAFFOLD: the lookup + HTTP round-trip are stubbed. Wire a real
//! client (the engine's REST dialect) where marked `TODO`.

use starter_ext_sdk::serde_json::{json, Value};

use crate::device::take_str;
use crate::extension::ControlEngineCtx;

/// Resolve a device's connection row from the catalog. Returns the
/// row map, or a validation error if the `device_id` is unknown.
///
/// TODO: query `com.nubeio.ce.device_get` and return the single row:
///
///   let rows = ctx.warehouse_read()
///       .query("com.nubeio.ce.device_get", json!({ "device_id": device_id }))?;
///   rows.into_iter().next().ok_or(...)
fn _resolve_engine(
    _ctx: &ControlEngineCtx,
    _device_id: &str,
) -> starter_ext_sdk::Result<Value> {
    Ok(json!({}))
}

/// `engine_status` — ping the engine and report reachability.
pub fn handle_status(_ctx: &ControlEngineCtx, params: &Value) -> starter_ext_sdk::Result<Value> {
    let device_id = take_str(params, "device_id", "engine_status")?;

    // TODO: resolve the engine row, open an HTTP client to
    // `https://{ip}:{port}/...`, hit its health endpoint, map the
    // result to { online, engine_kind, version }.
    let _ = device_id;
    Ok(json!({ "device_id": "", "online": false, "version": null }))
}

/// `engine_wiresheet_get` — fetch the engine's current program and
/// return it as a node/edge graph the wiresheet canvas can render.
pub fn handle_wiresheet_get(
    _ctx: &ControlEngineCtx,
    params: &Value,
) -> starter_ext_sdk::Result<Value> {
    let device_id = take_str(params, "device_id", "engine_wiresheet_get")?;

    // TODO: GET the program from the engine, translate its native
    // block representation into `{ nodes: [...], edges: [...] }` using
    // the slot/kind vocabulary the UI's BaseNode expects.
    let _ = device_id;
    Ok(json!({ "device_id": "", "nodes": [], "edges": [] }))
}

/// `engine_wiresheet_put` — push an edited node/edge graph back to the
/// engine, reprogramming it.
pub fn handle_wiresheet_put(
    _ctx: &ControlEngineCtx,
    params: &Value,
) -> starter_ext_sdk::Result<Value> {
    let device_id = take_str(params, "device_id", "engine_wiresheet_put")?;

    // TODO: translate the incoming `{ nodes, edges }` into the engine's
    // native program format and PUT/POST it, returning how many blocks
    // were written.
    let _ = device_id;
    Ok(json!({ "operation": "wiresheet_put", "affected": 0 }))
}

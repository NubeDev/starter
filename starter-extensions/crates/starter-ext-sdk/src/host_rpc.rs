//! Outbound JSON-RPC channel from a process-flavour child to its
//! supervisor.
//!
//! The process-flavour child SDK runs an async I/O loop in
//! [`crate::process::run_process_main`]: every inbound frame is either
//! a request from the supervisor (which the loop dispatches to the
//! extension's handler) or a response to an *outbound* request the
//! child issued to invoke a host method (e.g. `dashboard.read`).
//! [`HostRpc`] owns the outbound half of that split: the writer task
//! that serialises frames, the per-id pending-response map, and the
//! atomic id allocator the call-side hands ids out from.
//!
//! ## Sync facade
//!
//! [`HostRpc::call_sync`] is what the SDK's `Backend` impls (which
//! must satisfy a sync trait so extension code can call
//! `ctx.dashboard().read(...)` without `.await`) reach. It bridges
//! to async via `block_in_place` + `block_on` — safe because the
//! dispatch loop runs handlers on the same multi-threaded tokio
//! runtime, and the SDK doc-strings call out that capability
//! accessors may block briefly.
//!
//! ## Failure shape
//!
//! - JSON-RPC `error` payloads round-trip back to the caller as
//!   the SPI `Error` variant the host emitted.
//! - The writer task dropped (supervisor stdin closed) surfaces
//!   as `Error::Transport("host-rpc writer closed")`.
//! - A timeout never fires from this side — the supervisor's own
//!   `request_timeout` decides when an in-flight call gives up.
//!   The child blocks until a response (or transport failure)
//!   arrives.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::json;
use starter_ext_spi::jsonrpc::JSONRPC_VERSION;
use starter_ext_spi::{Error, Result};
use tokio::sync::{mpsc, oneshot};

/// Per-request id floor. Picked to be far above the supervisor's
/// own request ids (which live below `1_000_000`); see
/// `starter-ext-supervisor::supervisor::DISPATCH_ID_FLOOR`. The two
/// crates use disjoint id ranges so a child's outbound id never
/// collides with a supervisor-driven dispatch id on the same wire.
const CHILD_REQUEST_ID_FLOOR: i64 = 100_000_000;

/// Outbound JSON-RPC channel. Cheap to clone; every backend stashes
/// one and shares the underlying writer + pending map through `Arc`.
#[derive(Clone)]
pub struct HostRpc {
    inner: Arc<Inner>,
}

/// Pending-response map: id → oneshot the call-side awaits on.
type PendingMap = Arc<Mutex<HashMap<i64, oneshot::Sender<Result<serde_json::Value>>>>>;

struct Inner {
    /// Send half of the writer's frame channel. The writer task
    /// drains this and writes each value as a Content-Length-framed
    /// JSON-RPC envelope on stdout.
    writer: mpsc::UnboundedSender<serde_json::Value>,
    /// Per-id pending responses.
    pending: PendingMap,
    /// Monotonic request-id allocator.
    next_id: AtomicI64,
}

impl HostRpc {
    /// Construct a fresh `HostRpc` bound to the supplied writer
    /// channel. The writer task should drain `writer_rx` and frame
    /// each value via `starter_jsonrpc_stdio::write_frame`. The
    /// process loop creates one of these per child lifetime.
    pub fn new(writer: mpsc::UnboundedSender<serde_json::Value>, pending: PendingMap) -> Self {
        Self {
            inner: Arc::new(Inner {
                writer,
                pending,
                next_id: AtomicI64::new(CHILD_REQUEST_ID_FLOOR),
            }),
        }
    }

    /// Borrow the pending map. The process loop's frame demultiplexer
    /// removes entries when a response arrives and calls
    /// `oneshot::Sender::send` on the awaiting caller. Exposed so
    /// the loop can share the same `Arc<Mutex<_>>` without rebuilding
    /// it.
    pub fn pending(&self) -> &PendingMap {
        &self.inner.pending
    }

    /// Synchronous host call. Allocates an id, frames a JSON-RPC
    /// request, dispatches it through the writer, and blocks on the
    /// matching response. Returns the deserialised JSON `result`
    /// or the kernel `Error` from a JSON-RPC `error` payload.
    pub fn call_sync(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        // Outside the tokio runtime is a programmer error — the
        // SDK's `run_process_main` runs every handler on a tokio
        // worker, so we always have one.
        let handle = tokio::runtime::Handle::try_current()
            .map_err(|e| Error::transport(format!("no tokio runtime for host call: {e}")))?;
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        // Insert *before* sending so an immediate response (paranoid
        // case) doesn't race with the read side observing the entry.
        {
            let mut g = self.inner.pending.lock().expect("pending mutex poisoned");
            g.insert(id, tx);
        }
        let mut envelope = json!({
            "jsonrpc": JSONRPC_VERSION,
            "id": id,
            "method": method,
            "params": params,
        });
        // Re-stamp `_meta.caller` from the per-call task-local so
        // the supervisor's host-method handler sees the same
        // tenant the tools/ call ran under. Absent ⇒ supervisor
        // treats the call as a system frame.
        if let Some(caller) = crate::caller_local::current() {
            envelope["_meta"] = json!({ "caller": caller });
        }
        if self.inner.writer.send(envelope).is_err() {
            // Writer task dropped — drain the entry we just inserted
            // so a subsequent retry doesn't see a stale sender.
            if let Ok(mut g) = self.inner.pending.lock() {
                g.remove(&id);
            }
            return Err(Error::transport("host-rpc writer closed"));
        }
        // Block on the response. We're called from a sync trait
        // method (`DashboardBackend::read` etc.) so the
        // `block_in_place` + `block_on` pattern is needed to bridge.
        tokio::task::block_in_place(|| handle.block_on(rx))
            .map_err(|_| Error::transport("host-rpc response channel closed"))?
    }
}

impl std::fmt::Debug for HostRpc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostRpc").finish_non_exhaustive()
    }
}

/// Helper for the process loop: route an inbound JSON-RPC response
/// (a frame with `id` + (`result` or `error`)) into the pending
/// map. Returns `true` if the frame was consumed as a response;
/// `false` if it should fall through to the handler dispatch path.
pub fn route_response(pending: &PendingMap, frame: &serde_json::Value) -> bool {
    // Must have an id (numeric — we never allocate string ids).
    let Some(id) = frame.get("id").and_then(|v| v.as_i64()) else {
        return false;
    };
    // Only frames inside our id range belong to us. Below the floor
    // is either a supervisor health response (which never reaches
    // the child — those round-trip through the supervisor's own
    // pending map) or some other unrelated id — leave it for the
    // handler dispatch.
    if id < CHILD_REQUEST_ID_FLOOR {
        return false;
    }
    // Has it a result or an error? (Notifications and inbound
    // requests have neither.)
    let has_result = frame.get("result").is_some();
    let has_error = frame.get("error").is_some();
    if !has_result && !has_error {
        return false;
    }
    let sender = {
        let mut g = pending.lock().expect("pending mutex poisoned");
        g.remove(&id)
    };
    let Some(sender) = sender else {
        // Response for an id we don't know about — drop it. Could
        // happen if the caller bailed before the supervisor replied.
        return true;
    };
    let payload = if has_error {
        let err_val = frame
            .get("error")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let kernel_err = serde_json::from_value::<Error>(err_val.clone()).unwrap_or_else(|_| {
            Error::extension_internal(format!("host returned non-Error error payload: {err_val}"))
        });
        Err(kernel_err)
    } else {
        Ok(frame
            .get("result")
            .cloned()
            .unwrap_or(serde_json::Value::Null))
    };
    let _ = sender.send(payload);
    true
}

/// Construct a fresh pending map. The process loop creates one,
/// hands a clone to the `HostRpc`, and consults the same `Arc`
/// during inbound frame demultiplexing.
pub fn new_pending_map() -> PendingMap {
    Arc::new(Mutex::new(HashMap::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_response_ignores_frames_below_floor() {
        let pending = new_pending_map();
        let frame = json!({"jsonrpc": "2.0", "id": 1, "result": {}});
        assert!(!route_response(&pending, &frame));
    }

    #[test]
    fn route_response_ignores_notifications() {
        let pending = new_pending_map();
        let frame = json!({"jsonrpc": "2.0", "method": "tools/foo"});
        assert!(!route_response(&pending, &frame));
    }

    #[test]
    fn route_response_consumes_in_range_responses() {
        let pending = new_pending_map();
        let id = CHILD_REQUEST_ID_FLOOR + 7;
        let (tx, mut rx) = oneshot::channel();
        pending.lock().unwrap().insert(id, tx);
        let frame = json!({"jsonrpc": "2.0", "id": id, "result": {"ok": true}});
        assert!(route_response(&pending, &frame));
        let got = rx.try_recv().expect("response delivered");
        assert_eq!(got.unwrap(), json!({"ok": true}));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn call_sync_round_trips_via_simulated_supervisor() {
        // Simulate the supervisor by spawning a task that drains
        // the writer channel, sees a request, and pushes a
        // response back via `route_response`. Asserts the
        // round-trip end-to-end without touching stdout.
        let (writer_tx, mut writer_rx) = tokio::sync::mpsc::unbounded_channel();
        let pending = new_pending_map();
        let rpc = HostRpc::new(writer_tx, pending.clone());

        let pending_clone = pending.clone();
        // Supervisor stand-in.
        tokio::spawn(async move {
            let req = writer_rx.recv().await.expect("writer alive");
            let id = req
                .get("id")
                .and_then(|v| v.as_i64())
                .expect("request carries id");
            let resp = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {"hello": "world"},
            });
            route_response(&pending_clone, &resp);
        });

        // `call_sync` blocks the current thread on `block_in_place`,
        // which needs the multi-thread runtime — provided by the
        // `#[tokio::test(flavor = "multi_thread")]` attr.
        let res = tokio::task::spawn_blocking(move || rpc.call_sync("ping", json!({})))
            .await
            .expect("blocking joined")
            .expect("rpc ok");
        assert_eq!(res, json!({"hello": "world"}));
    }

    #[test]
    fn route_response_decodes_error_payload() {
        let pending = new_pending_map();
        let id = CHILD_REQUEST_ID_FLOOR + 9;
        let (tx, mut rx) = oneshot::channel();
        pending.lock().unwrap().insert(id, tx);
        let err = Error::capability("denied");
        let err_val = serde_json::to_value(&err).unwrap();
        let frame = json!({"jsonrpc": "2.0", "id": id, "error": err_val});
        assert!(route_response(&pending, &frame));
        let got = rx.try_recv().expect("delivered");
        let kind = got.expect_err("error variant");
        assert!(matches!(kind, Error::Capability(_)), "got {kind:?}");
    }
}

//! Extension-side library for `hello-cli`.
//!
//! Defines the [`Hello`] unit struct (so `#[derive(Extension)]` reads
//! `block.yaml` at this crate's compile time and provides the
//! `ExtensionMeta` shape every adapter expects) and two pure handler
//! functions:
//!
//! - [`greet`] — non-streaming. Takes the parsed args object and
//!   returns one JSON value.
//! - [`tick`] — streaming. Emits N tick events on the per-call
//!   `EventSender`, returning when either the count is exhausted or
//!   the host fires `Cancel` (the CLI adapter wires `SIGINT` to that).
//!
//! `main.rs` then registers both functions into a
//! [`starter_ext_cli::BuiltinCliRegistry`] keyed by the manifest's
//! cli ids.

use starter_ext_sdk::serde_json::{self, Value};
use starter_ext_sdk::Extension;

/// SCOPE R5: no fields. State lives in the host-provided Ctx.
#[derive(Extension)]
#[extension(manifest = "block.yaml")]
pub struct Hello;

// `contributes.tools` is empty in `block.yaml`, so the proc-macro emits
// a `HelloToolHandlers` trait with no methods. Implement it trivially
// so the generated ExtensionDispatch bound is satisfied.
starter_ext_sdk::requires! {
    name = HelloCtx,
    capabilities = [],
}

impl HelloToolHandlers for Hello {
    type Ctx = HelloCtx;
}

starter_ext_sdk::register_static_table! {
    extension: Hello,
    ctx: HelloCtx,
    instance: Hello,
}

// ---------------------------------------------------------------------------
// CLI-only handlers. Registered by `main.rs` into a BuiltinCliRegistry.
// These do *not* go through the proc-macro's dispatch path because CLI
// handlers are out of scope for the v0.1 proc-macro (see
// `starter-ext-cli`'s crate docs).
// ---------------------------------------------------------------------------

/// Non-streaming `hellocli-greet` handler.
pub fn greet(
    params: Value,
    _ctx: &starter_ext_sdk::ctx::CtxInner,
) -> starter_ext_sdk::Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("world");
    Ok(serde_json::json!({ "message": format!("hello, {name}") }))
}

/// Streaming `hellocli-tick` handler. Emits one event per logical
/// tick; honours cancellation between ticks.
pub fn tick(
    params: Value,
    ctx: &starter_ext_sdk::ctx::CtxInner,
) -> starter_ext_sdk::Result<()> {
    let count = params.get("count").and_then(Value::as_i64).unwrap_or(3);
    let label = params
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("tick")
        .to_owned();
    let sender = ctx.events().clone();
    for n in 0..count {
        if ctx.cancel().is_cancelled() {
            break;
        }
        // The stream id is allocated by the adapter; the per-event
        // shape is `{ stream_id, payload }`. Builtin handlers can use
        // any stream_id — the adapter rewrites it on the wire.
        let ev = starter_ext_sdk::ctx::Event {
            stream_id: starter_ext_sdk::StreamId("hello-cli".into()),
            payload: serde_json::json!({ "n": n, "label": label }),
        };
        // try_send so a slow consumer doesn't pin us; on a full
        // channel we'd drop, which the adapter would surface as
        // backpressure. For this example the channel (16) is plenty.
        if sender.try_send(ev).is_err() {
            // Receiver dropped — treat as cancel.
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Ok(())
}

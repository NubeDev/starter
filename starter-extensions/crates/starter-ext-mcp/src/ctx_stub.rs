//! Phase-1 stub backends for the SDK's `CtxInner`.
//!
//! Every per-capability handle the SDK exposes (`SecretsHandle`,
//! `HttpOutHandle`, `FsHandle`, `WallClockHandle`, `TracingHandle`)
//! requires a backend trait impl. The real implementations live in
//! later phases — the secret store wires up to `starter_spi::secrets`,
//! HTTP-out goes through a `reqwest` client, etc. For Phase 1 (this
//! stage) we only need to demonstrate that a builtin extension's
//! handler can be invoked through the MCP transport; concrete backends
//! are out of scope.
//!
//! Every stub returns `Error::Capability("…not wired in Phase 1")`. An
//! extension that *uses* a capability accessor under this stub will see
//! the error at runtime; an extension that does not touch capabilities
//! (e.g. `hello-builtin`'s echo tool) runs cleanly. That is the
//! correct Phase 1 surface — we are validating the kernel, not the
//! capability wiring.

use std::sync::Arc;

use starter_ext_sdk::ctx::{
    CtxInner, FsBackend, HttpOutBackend, NeverCancel, SecretsBackend, TracingBackend,
    WallClockBackend,
};
use starter_ext_spi::Error;
use tokio::sync::mpsc;

/// Build a `CtxInner` whose capability backends all return
/// `Error::Capability`. The event channel is a small bounded `mpsc` so
/// streaming handlers compile and run; downstream wiring (collecting
/// `stream.event` notifications onto the wire) lands with the supervisor
/// phase.
pub(crate) fn make_stub_ctx() -> CtxInner {
    let (tx, _rx) = mpsc::channel(16);
    CtxInner::new(
        tx,
        Arc::new(NeverCancel),
        Arc::new(StubSecrets),
        Arc::new(StubHttpOut),
        Arc::new(StubFs),
        Arc::new(StubWallClock),
        Arc::new(StubTracing),
    )
}

fn deny(category: &str) -> Error {
    Error::capability(format!(
        "{category}: backend not wired in Phase 1 (starter-ext-mcp ctx stub). \
         The kernel host is functional; capability wiring lands in the \
         supervisor / wasm / adapter follow-up phases."
    ))
}

#[derive(Debug)]
struct StubSecrets;
impl SecretsBackend for StubSecrets {
    fn get(&self, _name: &str) -> starter_ext_spi::Result<String> {
        Err(deny("secrets"))
    }
}

#[derive(Debug)]
struct StubHttpOut;
impl HttpOutBackend for StubHttpOut {
    fn request(&self, _req: serde_json::Value) -> starter_ext_spi::Result<serde_json::Value> {
        Err(deny("http_out"))
    }
}

#[derive(Debug)]
struct StubFs;
impl FsBackend for StubFs {
    fn read(&self, _path: &str) -> starter_ext_spi::Result<Vec<u8>> {
        Err(deny("fs"))
    }
}

#[derive(Debug)]
struct StubWallClock;
impl WallClockBackend for StubWallClock {
    fn now_unix_ms(&self) -> starter_ext_spi::Result<u64> {
        Err(deny("wall_clock"))
    }
}

#[derive(Debug)]
struct StubTracing;
impl TracingBackend for StubTracing {
    fn event(&self, _level: &str, _msg: &str, _fields: serde_json::Value) {
        // Tracing is fire-and-forget; swallow the event silently rather
        // than panicking — a Phase 1 extension that emits diagnostics is
        // not a failure mode worth surfacing through the adapter.
    }
}

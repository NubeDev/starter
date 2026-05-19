//! Per-call `Store<T>` data.
//!
//! Each [`crate::WasmHost::dispatch_tool`] invocation builds a fresh
//! `Store<InstanceState>` (SCOPE.md "Stateless per-call instantiation
//! in v0.1"). The state struct holds three things, in order of who
//! needs them:
//!
//! 1. A `WasiCtx` + `ResourceTable` (the wasmtime-wasi crate's
//!    `WasiView` impl is what the linker keys off — without these
//!    fields, the `wasi:io/streams` add-to-linker call panics at
//!    runtime).
//! 2. A [`crate::MemoryLimiter`] the `Store::limiter` closure borrows
//!    on every linear-memory growth attempt.
//! 3. Diagnostic fields the host reads back after the call returns —
//!    fuel consumed, deadline expiry observed — for tracing the
//!    capability gate from the admin endpoint.
//!
//! The struct is `pub` because the WIT-side `Host` trait
//! implementations (once wired in a later stage's adapter) take
//! `&mut InstanceState` directly.

use wasmtime::component::ResourceTable;
use wasmtime_wasi::{IoView, WasiCtx, WasiCtxBuilder, WasiView};

use crate::limits::MemoryLimiter;

/// Per-call store data. Built once per dispatch call and dropped when
/// the call returns — that's how "stateless per-call instantiation"
/// translates into Rust at the type level: there is no `Store` in
/// existence between calls.
pub struct InstanceState {
    /// WASI context. Always present — even a default-deny call has a
    /// `WasiCtx` because `wasi:io/streams` (which carries no
    /// capability, only resource bookkeeping) is always linked.
    pub wasi: WasiCtx,

    /// Component-model resource table. Required by every `WasiView`-
    /// keyed `add-to-linker` call regardless of whether any non-stream
    /// WASI interface is granted.
    pub table: ResourceTable,

    /// Per-call memory cap, installed on the `Store` via
    /// [`wasmtime::Store::limiter`].
    pub limiter: MemoryLimiter,
}

impl InstanceState {
    /// Build a new per-call state with an empty WASI ctx (no
    /// `inherit_stdio`, no preopens, no env vars — SCOPE R8
    /// default-deny). Granting capabilities happens by mutating
    /// `self.wasi` *before* the dispatch call begins; once the
    /// component starts executing, the ctx is fixed.
    pub fn new(limiter: MemoryLimiter) -> Self {
        // `WasiCtxBuilder` defaults to no inherited stdio, no preopens,
        // no env vars — exactly what R8 demands. We do not call any of
        // the `inherit_*` / `preopened_dir` / `env` methods here; the
        // `WasmHost` caller is what decides whether to do so, based on
        // the manifest's granted categories.
        let wasi = WasiCtxBuilder::new().build();
        let table = ResourceTable::new();
        Self {
            wasi,
            table,
            limiter,
        }
    }
}

impl std::fmt::Debug for InstanceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstanceState")
            .field("limiter", &self.limiter)
            .finish_non_exhaustive()
    }
}

impl IoView for InstanceState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl WasiView for InstanceState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_state_carries_an_empty_wasi_ctx() {
        // The empty WasiCtx is what guarantees default-deny — no env
        // vars, no preopens, no stdio. We cannot introspect the ctx
        // directly (its fields are private inside wasmtime-wasi), so
        // we settle for "the constructor accepts and returns" and rely
        // on the host crate's integration test (linker setup) to
        // exercise the link-time refusal end-to-end.
        let limiter = MemoryLimiter::new(1 << 20);
        let _state = InstanceState::new(limiter);
    }
}

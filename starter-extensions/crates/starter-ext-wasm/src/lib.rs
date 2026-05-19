//! # starter-ext-wasm
//!
//! Optional WASI-p2 component host for the `starter-extensions` workspace.
//! Kernel Phase 4 / Stage 11 — the last of the three packaging flavours
//! to land. Per SCOPE.md:
//!
//! - **R1 / "Phase 4 — `starter-ext-wasm`"**: WASM-flavour extensions are
//!   WASI-p2 components instantiated by this crate. The trait the
//!   extension implements is unchanged from the builtin and process
//!   flavours; only the entry-point glue (`wit_bindgen::export!` against
//!   `crates/starter-ext-wasm/wit/starter-extension.wit`) differs.
//! - **R8 / "Default-deny for WASM; explicit-grant"**: every component is
//!   instantiated with **no WASI capabilities**. The manifest's
//!   `capabilities:` block then adds explicit grants
//!   (`wasi:clocks/wall-clock`, `wasi:http/outgoing-handler`, …). A
//!   component that imports a WASI interface the host did not grant
//!   fails at instantiation — link-time enforcement, not advisory.
//! - **R8 / "Per-call fuel + memory + wall-clock caps"**: the cap values
//!   live in the host's [`Caps`] struct, *not* in the manifest. Operators
//!   tune them per deployment (a tight admin host gets megabytes of
//!   memory and a 100ms deadline; a long-running batch host gets
//!   gigabytes and a one-hour deadline). The manifest never expresses
//!   what wasmtime would refuse to enforce anyway.
//! - **"Stateless per-call instantiation in v0.1"**: every
//!   [`WasmHost::dispatch_tool`] call builds a fresh `Store` and a fresh
//!   component instance. The `kv` capability the WIT package reserves
//!   for v0.2 is what enables stateful per-extension instantiation later
//!   — until that lands, the per-call discipline is what guarantees one
//!   call cannot observe another's state.
//!
//! ## What this crate does *not* do
//!
//! - It does not parse manifests (`starter-ext-host`'s job — R2).
//! - It does not wire any transport adapter (`starter-ext-mcp` /
//!   `starter-ext-rest` / … per R13).
//! - It does not supervise long-running extensions (`starter-ext-supervisor`
//!   handles the process flavour; wasm components in v0.1 live for one
//!   dispatch call each).
//!
//! ## Module layout
//!
//! Per the workspace convention "one concept per `*.rs` file" (see
//! `crates/starter-ext-supervisor/src/lib.rs`):
//!
//! - [`caps`] — the [`Caps`] struct (per-call fuel + memory + deadline
//!   + capability grant set).
//! - [`limits`] — [`MemoryLimiter`], a [`wasmtime::ResourceLimiter`]
//!   honouring `Caps::max_memory_bytes`.
//! - [`state`] — [`InstanceState`], the per-call `Store<T>` data plus
//!   the `WasiView` impl.
//! - [`host`] — [`WasmHost`], the public surface. One `WasmHost` per
//!   consumer host; `dispatch_tool` per call.
//! - [`wit_package`] — the embedded `starter:extension@0.1.0` WIT package
//!   bytes the loader hashes for diagnostics.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod caps;
pub mod host;
pub mod limits;
pub mod state;
pub mod wit_package;

pub use caps::{Caps, WasiCategory};
pub use host::{WasmCallOutcome, WasmHost};
pub use limits::MemoryLimiter;
pub use state::InstanceState;
pub use wit_package::{WIT_PACKAGE, WIT_PACKAGE_NAME, WIT_PACKAGE_VERSION};

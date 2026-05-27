//! # starter-ext-supervisor
//!
//! Process-flavour lifecycle for the `starter-extensions` workspace.
//!
//! SCOPE.md Phase 2 ("the heaviest crate, gets the most attention"). The
//! supervisor's job is to take a `Validated` [`ExtensionRecord`] whose
//! manifest declares `runtime.kind: process`, spawn the child binary under
//! `runtime.bin`, frame stdio JSON-RPC via
//! [`starter_jsonrpc_stdio`] (the same crate `starter-mcp` consumes — one
//! framing implementation, no drift), drive the init handshake with
//! manifest content-hash verification (R3), and from there:
//!
//! - Restart according to the manifest's [`RestartPolicy`] with intensity
//!   cap + exponential backoff with jitter (R9 — no supervisor groups in
//!   v0.1; every extension restarts independently).
//! - Periodically ping the child via the `health` notification; missed
//!   pings count as crashes.
//! - Forward `stream.event` / `stream.end` / `stream.error` /
//!   `stream.cancel` notifications without interpretation (the kernel
//!   shape; adapters translate to their transport's native frames).
//! - Enforce capability use at the JSON-RPC *wire* boundary (advisory in
//!   v0.1 per R8 — the supervisor cannot prevent the child from doing
//!   something inside its own address space, but it can refuse to wire
//!   undeclared host methods through and count the violations).
//! - Surface diagnostics through a bounded [`EventRing`] per extension
//!   (state transitions, crash reasons, restart counts, last N stderr
//!   lines) and the `capability_violation` counter.
//! - On shutdown: send `SIGTERM`, wait the manifest's
//!   `supervision.shutdown_grace_ms`, then `SIGKILL` if the child has not
//!   exited.
//!
//! The crate is deliberately small in surface — the heavy lifting lives
//! in the submodules and the supervisor exposes one entry point per
//! external concern. Each submodule is its own file so the next session
//! reading this crate can locate one concept per `*.rs` file.
//!
//! [`ExtensionRecord`]: starter_ext_host::ExtensionRecord
//! [`RestartPolicy`]: starter_ext_spi::RestartPolicy

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod backoff;
pub mod caller_local;
pub mod capability;
pub mod event_ring;
pub mod handshake;
pub mod host_methods;
pub mod restart;
pub mod stream;
pub mod supervisor;

pub use backoff::BackoffSchedule;
pub use capability::{CapabilityGate, CapabilityViolationCounter, CAPABILITY_HOST_METHODS};
pub use event_ring::{Event as RingEvent, EventKind, EventRing};
pub use handshake::{manifest_hash, InitHandshake, InitReady};
pub use host_methods::{HostMethodHandler, NotImplementedHandler, SharedHostMethodHandler};
pub use restart::{RestartDecision, RestartTracker};
pub use stream::is_streaming_notification;
pub use supervisor::{ShutdownReason, Supervisor, SupervisorHandle};

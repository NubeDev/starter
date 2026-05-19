//! Long-running `Service` shape. The companion to the existing `Tool`
//! trait (see `crate::tool`).
//!
//! A `Tool` is a one-shot request/response — the caller hands input in,
//! the implementation hands a value back, and starter-mcp / a REST
//! handler observes the result. A `Service` has no caller: it runs on
//! its own (socket-mode listener, long-poll loop, webhook receiver
//! background task, …) and publishes everything it observes into an
//! [`EventSink`]. Its error path is observability-only — the registry
//! does not auto-restart (SCOPE rule R9).
//!
//! What this module contributes to `starter-spi`:
//!
//! - [`Service`] trait + [`ServiceContext`] + [`ServiceHandle`] —
//!   the lifecycle contract.
//! - [`ServiceRegistry`] — mirrors `ToolRegistry`'s shape, owns the
//!   single `tokio::sync::watch::Sender<bool>` and fans out receivers
//!   to every spawned service (R2 — no service holds the sender).
//! - [`EventSink`] trait + [`SinkError`] + [`Event`] + a
//!   [`Vec<Arc<dyn EventSink>>`](FanOut) fan-out helper.
//! - With the default-on `broadcast` Cargo feature, a blanket
//!   [`EventSink`] impl for `tokio::sync::broadcast::Sender<E>`
//!   (Decision D2).
//!
//! See `DOCS/tools/scope/SCOPE.md` rules R2, R4, R8, R9 and decisions
//! D2, D3, D4.

mod context;
mod event;
mod fanout;
mod handle;
mod registry;
#[allow(clippy::module_inception)]
mod service;
mod sink;

#[cfg(feature = "broadcast")]
pub mod broadcast;

pub use context::ServiceContext;
pub use event::Event;
pub use fanout::FanOut;
pub use handle::ServiceHandle;
pub use registry::{
    ServiceRegistry, ServiceShutdownOutcome, ShutdownReport, SHUTDOWN_DEADLINE_DEFAULT,
};
pub use service::Service;
pub use sink::{EventSink, SinkError, SinkResult};
